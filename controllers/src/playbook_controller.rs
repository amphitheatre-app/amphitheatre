// Copyright 2023 The Amphitheatre Authors.
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

use amp_common::schema::{Playbook, PlaybookState, Source};
use amp_resolver as resolver;
use amp_resources::event::trace;
use amp_resources::{actor, namespace, playbook};
use futures::{future, StreamExt};
use k8s_openapi::api::core::v1::ObjectReference;
use kube::api::ListParams;
use kube::runtime::controller::Action;
use kube::runtime::events::Recorder;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::runtime::Controller;
use kube::{Api, Resource, ResourceExt};

use crate::context::Context;
use crate::error::{Error, Result};

pub async fn new(ctx: &Arc<Context>) {
    let api = Api::<Playbook>::all(ctx.k8s.clone());

    // Ensure Playbook CRD is installed before loop-watching
    if let Err(e) = api.list(&ListParams::default().limit(1)).await {
        tracing::error!("Playbook CRD is not queryable; {e:?}. Is the CRD installed?");
        tracing::info!("Installation: amp-crdgen | kubectl apply -f -");
        std::process::exit(1);
    }

    Controller::new(api, ListParams::default())
        .run(reconcile, error_policy, ctx.clone())
        .for_each(|_| future::ready(()))
        .await
}

/// The reconciler that will be called when either object change
pub async fn reconcile(playbook: Arc<Playbook>, ctx: Arc<Context>) -> Result<Action> {
    tracing::info!("Reconciling Playbook \"{}\"", playbook.name_any());

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
    tracing::error!("reconcile failed: {:?}", error);
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
    namespace::create(ctx.k8s.clone(), playbook)
        .await
        .map_err(Error::ResourceError)?;
    trace(recorder, "Created namespace for this playbook")
        .await
        .map_err(Error::ResourceError)?;

    trace(recorder, "Init successfully, Let's begin resolving, now!")
        .await
        .map_err(Error::ResourceError)?;
    playbook::patch_status(&ctx.k8s, playbook, PlaybookState::resolving())
        .await
        .map_err(Error::ResourceError)?;

    Ok(())
}

async fn resolve(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    let mut fetches: HashSet<Source> = HashSet::new();

    if let Some(actors) = &playbook.spec.actors {
        let exists: HashSet<&String> = actors.iter().map(|actor| &actor.name).collect();

        for actor in actors {
            if let Some(partners) = &actor.partners {
                for (name, partner) in partners {
                    if exists.contains(name) {
                        continue;
                    }
                    fetches.insert(partner.clone());
                }
            }
        }

        tracing::debug!("The currently existing actors are: {exists:#?}");
    } else {
        tracing::debug!("Build from the starting characters (preface)");
        fetches.insert(playbook.spec.preface.clone());
    }

    tracing::debug!("The repositories to be fetched are: {fetches:#?}");
    let configuration = ctx.configuration.read().await;

    for source in fetches.iter() {
        tracing::info!("fetching partner with source: {}", source.uri());
        let actor = resolver::load(&configuration, source).map_err(Error::ResolveError)?;

        trace(recorder, "Fetch and add the actor to this playbook")
            .await
            .map_err(Error::ResourceError)?;
        playbook::add(&ctx.k8s, playbook, actor)
            .await
            .map_err(Error::ResourceError)?;
    }

    if fetches.is_empty() {
        trace(recorder, "Resolved successfully, Running")
            .await
            .map_err(Error::ResourceError)?;
        playbook::patch_status(&ctx.k8s, playbook, PlaybookState::running(true, "AutoRun", None))
            .await
            .map_err(Error::ResourceError)?;
    }

    Ok(())
}

async fn run(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    if let Some(actors) = &playbook.spec.actors {
        for spec in actors {
            match actor::exists(ctx.k8s.clone(), playbook, spec)
                .await
                .map_err(Error::ResourceError)?
            {
                true => {
                    // Actor already exists, update it if there are new changes
                    trace(
                        recorder,
                        format!(
                            "Actor {} already exists, update it if there are new changes",
                            spec.name
                        ),
                    )
                    .await
                    .map_err(Error::ResourceError)?;
                    actor::update(ctx.k8s.clone(), playbook, spec)
                        .await
                        .map_err(Error::ResourceError)?;
                }
                false => {
                    // Create a new actor
                    trace(recorder, format!("Create new Actor: {}", spec.name))
                        .await
                        .map_err(Error::ResourceError)?;
                    actor::create(ctx.k8s.clone(), playbook, spec)
                        .await
                        .map_err(Error::ResourceError)?;
                }
            }
        }
    }
    Ok(())
}

pub async fn cleanup(_playboo: &Playbook, _ctx: &Arc<Context>, _recorder: &Recorder) -> Result<Action> {
    Ok(Action::await_change())
}

#[inline]
fn reference(playbbok: &Playbook) -> ObjectReference {
    let mut reference = playbbok.object_ref(&());
    reference.namespace = Some(playbbok.spec.namespace.to_string());
    reference
}
