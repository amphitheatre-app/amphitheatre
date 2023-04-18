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

use std::sync::Arc;
use std::time::Duration;

use amp_common::docker::{self, registry, DockerConfig};
use amp_common::schema::{Actor, ActorState};
use amp_resources::event::trace;
use amp_resources::{actor, deployment, image, job, service};
use futures::{future, StreamExt};
use k8s_openapi::api::core::v1::Namespace;
use kube::api::ListParams;
use kube::runtime::controller::Action;
use kube::runtime::events::Recorder;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::runtime::Controller;
use kube::{Api, Resource, ResourceExt};
use tracing::{error, info};

use crate::context::Context;
use crate::error::{Error, Result};

pub async fn new(ctx: &Arc<Context>) {
    let api = Api::<Actor>::all(ctx.k8s.clone());

    // Ensure Actor CRD is installed before loop-watching
    if let Err(e) = api.list(&ListParams::default().limit(1)).await {
        tracing::error!("Actor CRD is not queryable; {e:?}. Is the CRD installed?");
        tracing::info!("Installation: amp-crdgen | kubectl apply -f -");
        std::process::exit(1);
    }

    Controller::new(api, ListParams::default())
        .run(reconcile, error_policy, ctx.clone())
        .for_each(|_| future::ready(()))
        .await
}

/// The reconciler that will be called when either object change
pub async fn reconcile(actor: Arc<Actor>, ctx: Arc<Context>) -> Result<Action> {
    tracing::info!("Reconciling Actor \"{}\"", actor.name_any());

    let ns = actor.namespace().unwrap(); // actor is namespace scoped
    let api: Api<Actor> = Api::namespaced(ctx.k8s.clone(), &ns);
    let recorder = ctx.recorder(actor.object_ref(&()));

    // Reconcile the actor custom resource.
    let finalizer_name = "actors.amphitheatre.app/finalizer";
    finalizer(&api, finalizer_name, actor, |event| async {
        match event {
            FinalizerEvent::Apply(actor) => apply(&actor, &ctx, &recorder).await,
            FinalizerEvent::Cleanup(actor) => cleanup(&actor, &ctx, &recorder).await,
        }
    })
    .await
    .map_err(|e| Error::FinalizerError(Box::new(e)))
}
/// an error handler that will be called when the reconciler fails with access to both the
/// object that caused the failure and the actual error
pub fn error_policy(_actor: Arc<Actor>, error: &Error, _ctx: Arc<Context>) -> Action {
    tracing::error!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(60))
}

async fn apply(actor: &Actor, ctx: &Arc<Context>, recorder: &Recorder) -> Result<Action> {
    if let Some(ref status) = actor.status {
        if status.pending() {
            init(actor, ctx, recorder).await?
        } else if status.building() {
            build(actor, ctx, recorder).await?
        } else if status.running() {
            run(actor, ctx, recorder).await?
        }
    }

    Ok(Action::await_change())
    // If no events were received, check back every 2 minutes
    // Ok(Action::requeue(Duration::from_secs(2 * 60)))
}

async fn init(actor: &Actor, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    trace(recorder, format!("Building the image for Actor {}", actor.name_any()))
        .await
        .map_err(Error::ResourceError)?;
    actor::patch_status(&ctx.k8s, actor, ActorState::building())
        .await
        .map_err(Error::ResourceError)?;
    Ok(())
}

async fn build(actor: &Actor, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    // Return if the image already exists
    let configuration = ctx.configuration.read().await;
    let config = DockerConfig::from(&configuration.registry);

    let credential = docker::get_credential(&config, &actor.spec.image);
    let credential = match credential {
        Ok(credential) => Some(credential),
        Err(err) => {
            error!("Error handling docker configuration: {}", err);
            None
        }
    };

    if registry::exists(&actor.spec.image, credential)
        .await
        .map_err(Error::DockerRegistryExistsFailed)?
    {
        info!("The images already exists, Running");
        let condition = ActorState::running(true, "AutoRun", None);
        actor::patch_status(&ctx.k8s, actor, condition)
            .await
            .map_err(Error::ResourceError)?;

        return Ok(());
    }

    // Prefer to use Kaniko to build images with Dockerfile,
    // else, build the image with Cloud Native Buildpacks
    if actor.spec.has_dockerfile() {
        build_with_kaniko(actor, ctx, recorder).await?;
    } else {
        build_with_kpack(actor, ctx, recorder).await?;
    }

    // Check If the build Job has not completed, requeue the reconciler.
    if !job::completed(&ctx.k8s, actor).await.map_err(Error::ResourceError)? {
        return Ok(());
    }

    // Once the image is built, it is deployed to the cluster with the
    // appropriate resource type (e.g. Deployment or StatefulSet).
    trace(recorder, "The images builded, Running")
        .await
        .map_err(Error::ResourceError)?;
    let condition = ActorState::running(true, "AutoRun", None);
    actor::patch_status(&ctx.k8s, actor, condition)
        .await
        .map_err(Error::ResourceError)?;

    Ok(())
}

async fn build_with_kaniko(actor: &Actor, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    match job::exists(&ctx.k8s, actor).await.map_err(Error::ResourceError)? {
        true => {
            // Build job already exists, update it if there are new changes
            trace(
                recorder,
                format!(
                    "Build job {} already exists, update it if there are new changes",
                    actor.spec.build_name()
                ),
            )
            .await
            .map_err(Error::ResourceError)?;
            job::update(&ctx.k8s, actor).await.map_err(Error::ResourceError)?;
        }
        false => {
            // Create a new build job
            trace(recorder, format!("Create new build Job: {}", actor.spec.build_name()))
                .await
                .map_err(Error::ResourceError)?;
            job::create(&ctx.k8s, actor).await.map_err(Error::ResourceError)?;
        }
    }

    Ok(())
}

async fn build_with_kpack(actor: &Actor, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    match image::exists(&ctx.k8s, actor).await.map_err(Error::ResourceError)? {
        true => {
            // Image already exists, update it if there are new changes
            trace(
                recorder,
                format!(
                    "Image {} already exists, update it if there are new changes",
                    actor.spec.build_name()
                ),
            )
            .await
            .map_err(Error::ResourceError)?;
            image::update(&ctx.k8s, actor).await.map_err(Error::ResourceError)?;
        }
        false => {
            // Create a new image
            trace(recorder, format!("Create new image: {}", actor.spec.build_name()))
                .await
                .map_err(Error::ResourceError)?;
            image::create(&ctx.k8s, actor).await.map_err(Error::ResourceError)?;
        }
    }

    Ok(())
}

async fn run(actor: &Actor, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    trace(
        recorder,
        format!("Try to deploying the resources for Actor {}", actor.name_any()),
    )
    .await
    .map_err(Error::ResourceError)?;

    match deployment::exists(&ctx.k8s, actor)
        .await
        .map_err(Error::ResourceError)?
    {
        true => {
            // Deployment already exists, update it if there are new changes
            trace(
                recorder,
                format!(
                    "Deployment {} already exists, update it if there are new changes",
                    actor.name_any()
                ),
            )
            .await
            .map_err(Error::ResourceError)?;
            deployment::update(&ctx.k8s, actor)
                .await
                .map_err(Error::ResourceError)?;
        }
        false => {
            // Create a new Deployment
            trace(recorder, format!("Create new Deployment: {}", actor.name_any()))
                .await
                .map_err(Error::ResourceError)?;
            deployment::create(&ctx.k8s, actor)
                .await
                .map_err(Error::ResourceError)?;
        }
    }

    if actor.spec.service_ports().is_some() {
        match service::exists(&ctx.k8s, actor).await.map_err(Error::ResourceError)? {
            true => {
                // Service already exists, update it if there are new changes
                trace(
                    recorder,
                    format!(
                        "Service {} already exists, update it if there are new changes",
                        actor.name_any()
                    ),
                )
                .await
                .map_err(Error::ResourceError)?;
                service::update(&ctx.k8s, actor).await.map_err(Error::ResourceError)?;
            }
            false => {
                // Create a new Service
                trace(recorder, format!("Create new Service: {}", actor.name_any()))
                    .await
                    .map_err(Error::ResourceError)?;
                service::create(&ctx.k8s, actor).await.map_err(Error::ResourceError)?;
            }
        }
    }

    Ok(())
}

pub async fn cleanup(actor: &Actor, ctx: &Arc<Context>, recorder: &Recorder) -> Result<Action> {
    let namespace = actor.namespace().unwrap();
    let api: Api<Namespace> = Api::all(ctx.k8s.clone());

    let ns = api.get(namespace.as_str()).await.map_err(Error::KubeError)?;
    if let Some(status) = ns.status {
        if status.phase == Some("Terminating".into()) {
            return Ok(Action::await_change());
        }
    }

    trace(recorder, format!("Delete Actor `{}`", actor.name_any()))
        .await
        .map_err(Error::ResourceError)?;
    Ok(Action::await_change())
}
