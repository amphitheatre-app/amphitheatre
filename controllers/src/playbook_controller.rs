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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use amp_common::config::{Configuration, Credential};
use amp_common::docker::build_docker_config;
use amp_crds::actor::{ActorSpec, Build, Partner};
use amp_crds::playbook::{Playbook, PlaybookState};
use amp_resources::error::{Error, Result};
use amp_resources::event::trace;
use amp_resources::{actor, namespace, playbook, secret, service_account};
use futures::{future, StreamExt};
use k8s_openapi::api::core::v1::ObjectReference;
use kube::api::ListParams;
use kube::runtime::controller::Action;
use kube::runtime::events::Recorder;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::runtime::Controller;
use kube::{Api, Resource, ResourceExt};

use crate::context::Context;

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
    if playbook.spec.actors.is_empty() {
        return Err(Error::EmptyActorsError);
    }

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
        ctx.config.registry_username.clone(),
        ctx.config.registry_password.clone(),
    );

    let configuration = Configuration {
        registry: HashMap::from([(ctx.config.registry_url.clone(), credential)]),
        repositories: HashMap::default(),
    };
    trace(recorder, format!("The Configuration is {:#?}", &configuration)).await?;

    let mut secrets = vec![];

    // Create Docker registry secrets.
    let docker_config = build_docker_config(&configuration.registry);
    let registry_secret = secret::create_registry_secret(&ctx.k8s, namespace, docker_config).await?;
    secrets.push(registry_secret.clone());

    trace(
        recorder,
        format!(
            "Created Secret for Docker Registry Credential: {:#?}",
            registry_secret.name_any()
        ),
    )
    .await?;

    // Create repository secrets.
    for (endpoint, credential) in configuration.repositories.iter() {
        let secret = secret::create_repository_secret(&ctx.k8s, namespace, endpoint, credential).await?;
        secrets.push(secret.clone());
        trace(
            recorder,
            format!("Created Secret for repository: {:#?}", endpoint),
        )
        .await?;
    }

    // Patch this credentials to default service account
    trace(recorder, "Patch the credentials to default service account").await?;
    service_account::patch(ctx.k8s.clone(), namespace, "default", &secrets, true, true).await?;

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
