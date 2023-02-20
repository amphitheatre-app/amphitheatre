// Copyright 2022 The Amphitheatre Authors.
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

use amp_crds::actor::{ActorSpec, Build, Partner};
use amp_crds::playbook::{Playbook, PlaybookState};
use amp_resources::error::{Error, Result};
use amp_resources::event::trace;
use amp_resources::secret::{self, Credential, Kind};
use amp_resources::{actor, namespace, playbook, service_account};
use k8s_openapi::api::core::v1::ObjectReference;
use kube::runtime::controller::Action;
use kube::runtime::events::Recorder;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::{Api, Resource, ResourceExt};
use url::Url;

use crate::context::Context;

/// The reconciler that will be called when either object change
pub async fn reconciler(playbook: Arc<Playbook>, ctx: Arc<Context>) -> Result<Action> {
    tracing::info!("Reconciling Playbook \"{}\"", playbook.name_any());
    if playbook.spec.actors.is_empty() {
        return Err(Error::EmptyActorsError);
    }

    let api: Api<Playbook> = Api::all(ctx.k8s.clone());
    let recorder = ctx.recorder(reference(&playbook));

    // Reconcile the playbook custom resource.
    let finalizer_name = "playbooks.amphitheatre.app/finalizer";
    finalizer(&api, finalizer_name, playbook, |event| async {
        match event {
            FinalizerEvent::Apply(playbook) => reconcile(&playbook, &ctx, &recorder).await,
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

async fn reconcile(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<Action> {
    if let Some(ref status) = playbook.status {
        if status.pending() {
            init(playbook, ctx, recorder).await?
        } else if status.solving() {
            solve(playbook, ctx, recorder).await?
        } else if status.running() {
            run(playbook, ctx, recorder).await?
        }
    }

    Ok(Action::await_change())
    // If no events were received, check back every 2 minutes
    // Ok(Action::requeue(Duration::from_secs(2 * 60)))
}

/// Init create namespace, credentials and service accounts
async fn init(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    let namespace = &playbook.spec.namespace;

    // Create namespace for this playbook
    namespace::create(ctx.k8s.clone(), playbook).await?;
    trace(recorder, "Created namespace for this playbook").await?;

    // Docker registry Credential
    let credential = Credential::basic(
        Kind::Image,
        Url::parse(&ctx.config.registry_url).map_err(Error::UrlParseError)?,
        ctx.config.registry_username.clone(),
        ctx.config.registry_password.clone(),
    );

    trace(recorder, "Creating Secret for Docker Registry Credential").await?;
    secret::create(ctx.k8s.clone(), namespace.clone(), &credential).await?;

    // Patch this credential to default service account
    trace(recorder, "Patch the credential to default service account").await?;
    service_account::patch(ctx.k8s.clone(), namespace, "default", &credential, true, true).await?;

    trace(recorder, "Init successfully, Let's begin solve, now!").await?;
    playbook::patch_status(ctx.k8s.clone(), playbook, PlaybookState::solving()).await?;

    Ok(())
}

async fn solve(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    let exists: HashSet<String> = playbook.spec.actors.iter().map(|actor| actor.url()).collect();

    let mut fetches: HashSet<Partner> = HashSet::new();
    for actor in &playbook.spec.actors {
        if let Some(partners) = &actor.partners {
            for partner in partners {
                if exists.contains(&partner.url()) {
                    continue;
                }
                fetches.insert(partner.clone());
            }
        }
    }

    tracing::debug!("Existing repos are:\n{exists:#?}\nand fetches are: {fetches:#?}");

    for partner in fetches.iter() {
        tracing::info!("fetches url: {}", partner.url());
        let actor = read(ctx, partner).await?.unwrap();

        trace(recorder, "Fetch and add the actor to this playbook").await?;
        playbook::add(ctx.k8s.clone(), playbook, actor).await?;
    }

    if fetches.is_empty() {
        trace(recorder, "Solved successfully, Running").await?;
        playbook::patch_status(
            ctx.k8s.clone(),
            playbook,
            PlaybookState::running(true, "AutoRun", None),
        )
        .await?;
    }

    Ok(())
}

async fn run(playbook: &Playbook, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    for spec in &playbook.spec.actors {
        match actor::exists(ctx.k8s.clone(), playbook, spec).await? {
            true => {
                // Actor already exists, update it if there are new changes
                trace(
                    recorder,
                    format!(
                        "Actor {} already exists, update it if there are new changes",
                        spec.name
                    ),
                )
                .await?;
                actor::update(ctx.k8s.clone(), playbook, spec).await?;
            }
            false => {
                // Create a new actor
                trace(recorder, format!("Create new Actor: {}", spec.name)).await?;
                actor::create(ctx.k8s.clone(), playbook, spec).await?;
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

// TODO: Read real actor information from remote VCS (like github).
pub async fn read(ctx: &Arc<Context>, partner: &Partner) -> Result<Option<ActorSpec>> {
    let spec = ActorSpec {
        name: partner.name.clone(),
        description: "A simple Golang example app".into(),
        image: format!("{}/{}", ctx.config.registry_namespace, "amp-example-go"),
        repository: partner.repository.clone(),
        reference: partner.reference.clone(),
        path: partner.path.clone(),
        commit: "2ebf3c7954f34e4a59976fdff985ea12a2009a52".into(),
        build: Some(Build {
            dockerfile: Some("Dockerfile".to_string()),
            ..Default::default()
        }),
        ..ActorSpec::default()
    };

    Ok(Some(spec))
}
