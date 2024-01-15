// Copyright (c) The Amphitheatre Authors. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use amp_common::resource::{Partner, Playbook, PlaybookState};
use amp_resolver as resolver;
use amp_resources::event::trace;
use amp_resources::{actor, namespace, playbook};
use futures::{future, StreamExt};
use k8s_openapi::api::core::v1::ObjectReference;
use kube::api::ListParams;
use kube::runtime::controller::Action;
use kube::runtime::events::Recorder;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::runtime::{watcher, Controller};
use kube::{Api, Resource, ResourceExt};
use tracing::{debug, error, info};

use crate::context::Context;
use crate::error::{Error, Result};

pub async fn new(ctx: &Arc<Context>) {
    let api = Api::<Playbook>::all(ctx.k8s.clone());

    // Ensure Playbook CRD is installed before loop-watching
    if let Err(e) = api.list(&ListParams::default().limit(1)).await {
        error!("Playbook CRD is not queryable; {e:?}. Is the CRD installed?");
        info!("Installation: amp-crdgen | kubectl apply -f -");
        std::process::exit(1);
    }

    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx.clone())
        .for_each(|_| future::ready(()))
        .await
}

/// The reconciler that will be called when either object change
pub async fn reconcile(playbook: Arc<Playbook>, ctx: Arc<Context>) -> Result<Action> {
    info!("Reconciling Playbook \"{}\"", playbook.name_any());

    let api: Api<Playbook> = Api::all(ctx.k8s.clone());
    let recorder = ctx.recorder(reference(&playbook));

    // Reconcile the playbook custom resource.
    let finalizer_name = "playbooks.amphitheatre.app/finalizer";
    finalizer(&api, finalizer_name, playbook, |event| async {
        match event {
            FinalizerEvent::Apply(playbook) => apply(&playbook, &ctx, &recorder).await,
            FinalizerEvent::Cleanup(playbook) => cleanup(&playbook, &ctx, &recorder).await,
        }
    })
    .await
    .map_err(|e| Error::FinalizerError(Box::new(e)))
}

/// an error handler that will be called when the reconciler fails with access to both the
/// object that caused the failure and the actual error
pub fn error_policy(_playbook: Arc<Playbook>, error: &Error, _ctx: Arc<Context>) -> Action {
    error!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(60))
}

async fn apply(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<Action> {
    if let Some(ref status) = playbook.status {
        if status.pending() {
            init(playbook, ctx, recorder).await?
        } else if status.resolving() {
            resolve(playbook, ctx, recorder).await?
        } else if status.running() {
            run(playbook, ctx, recorder).await?
        }
    }

    Ok(Action::await_change())
}

/// Init create namespace and go to resolving.
async fn init(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    // Create namespace for this playbook
    namespace::create(&ctx.k8s, playbook).await.map_err(Error::ResourceError)?;
    trace(recorder, "Created namespace for this playbook").await;

    add_preface(playbook, ctx, recorder).await?;
    playbook::patch_status(&ctx.k8s, playbook, PlaybookState::resolving()).await.map_err(Error::ResourceError)?;
    trace(recorder, "Init successfully, Let's begin resolving, now!").await;

    Ok(())
}

async fn resolve(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    // Check if there are any repositories to fetch
    //
    let mut fetches: HashSet<(&str, Partner)> = HashSet::new();

    if let Some(characters) = &playbook.spec.characters {
        let exists: HashSet<&String> = characters.iter().map(|char| &char.meta.name).collect();
        debug!("The currently existing actors are: {exists:?}");

        for character in characters {
            if let Some(partners) = &character.partners {
                for (name, partner) in partners {
                    if !exists.contains(name) {
                        fetches.insert((name, partner.clone()));
                    }
                }
            }
        }

        debug!("The repositories to be fetched are: {fetches:?}");
    }

    // Fetch the actors from the repositories
    //
    let credentials: tokio::sync::RwLockReadGuard<'_, amp_common::config::Credentials> = ctx.credentials.read().await;
    for (name, partner) in fetches.iter() {
        let character =
            resolver::partner::load(&ctx.k8s, &credentials, name, partner).await.map_err(Error::ResolveError)?;
        playbook::add(&ctx.k8s, playbook, character).await.map_err(Error::ResourceError)?;
        trace(recorder, "Fetch and add the actor to this playbook").await;
    }

    // If there are no repositories to fetch, then the resolution is complete.
    if fetches.is_empty() {
        let condition = PlaybookState::running(true, "AutoRun", None);
        playbook::patch_status(&ctx.k8s, playbook, condition).await.map_err(Error::ResourceError)?;
        trace(recorder, "Resolved successfully, Running").await;
    }

    Ok(())
}

async fn add_preface(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    debug!("Build from the starting characters (preface)");

    let credentials = ctx.credentials.read().await;
    let character =
        resolver::preface::load(&ctx.k8s, &credentials, &playbook.spec.preface).await.map_err(Error::ResolveError)?;
    playbook::add(&ctx.k8s, playbook, character).await.map_err(Error::ResourceError)?;
    trace(recorder, "Fetch and add the character to this playbook").await;

    Ok(())
}

async fn run(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    let credentials = ctx.credentials.read().await;

    if let Some(characters) = &playbook.spec.characters {
        for character in characters {
            let name = &character.meta.name;
            match actor::exists(&ctx.k8s, playbook, name).await.map_err(Error::ResourceError)? {
                true => {
                    // Actor already exists, update it if there are new changes
                    trace(recorder, format!("Try to refresh an existing Actor {}", name)).await;

                    let spec = resolver::to_actor(character, &credentials).map_err(Error::ResolveError)?;
                    actor::update(&ctx.k8s, playbook, &spec).await.map_err(Error::ResourceError)?;
                }
                false => {
                    // Create a new actor
                    trace(recorder, format!("Create new Actor: {}", name)).await;

                    let spec = resolver::to_actor(character, &credentials).map_err(Error::ResolveError)?;
                    actor::create(&ctx.k8s, playbook, &spec).await.map_err(Error::ResourceError)?;
                }
            }
        }
    }
    Ok(())
}

pub async fn cleanup(playbook: &Playbook, ctx: &Arc<Context>, _recorder: &Recorder) -> Result<Action> {
    // Try to delete the NATS stream for this playbook if it exists.
    if ctx.jetstream.delete_stream(playbook.name_any()).await.is_ok() {
        debug!("Deleted NATS stream for playbook {}", playbook.name_any());
    }

    Ok(Action::await_change())
}

#[inline]
fn reference(playbook: &Playbook) -> ObjectReference {
    let mut reference = playbook.object_ref(&());
    reference.namespace = Some(playbook.spec.namespace());
    reference
}
