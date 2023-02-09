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

use k8s_openapi::api::core::v1::{Namespace, ObjectReference};
use kube::runtime::controller::Action;
use kube::runtime::events::Recorder;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::{Api, Resource, ResourceExt};

use crate::context::Context;
use crate::resources::crds::{Actor, ActorState};
use crate::resources::error::{Error, Result};
use crate::resources::event::trace;
use crate::resources::{actor, deployment, image, job, service};

/// The reconciler that will be called when either object change
pub async fn reconcile(actor: Arc<Actor>, ctx: Arc<Context>) -> Result<Action> {
    tracing::info!("Reconciling Actor \"{}\"", actor.name_any());

    let ns = actor.namespace().unwrap(); // actor is namespace scoped
    let api: Api<Actor> = Api::namespaced(ctx.k8s.clone(), &ns);

    // Reconcile the actor custom resource.
    let finalizer_name = "actors.amphitheatre.app/finalizer";
    finalizer(&api, finalizer_name, actor, |event| async {
        match event {
            FinalizerEvent::Apply(actor) => actor.reconcile(ctx.clone()).await,
            FinalizerEvent::Cleanup(actor) => actor.cleanup(ctx.clone()).await,
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

impl Actor {
    pub async fn reconcile(&self, ctx: Arc<Context>) -> Result<Action> {
        if let Some(ref status) = self.status {
            if status.pending() {
                self.init(ctx).await?
            } else if status.building() {
                self.build(ctx).await?
            } else if status.running() {
                self.run(ctx).await?
            }
        }

        Ok(Action::await_change())
        // If no events were received, check back every 2 minutes
        // Ok(Action::requeue(Duration::from_secs(2 * 60)))
    }

    async fn init(&self, ctx: Arc<Context>) -> Result<()> {
        let recorder = ctx.recorder(self.reference());
        trace(
            &recorder,
            format!("Building the image for Actor {}", self.name_any()),
        )
        .await?;

        actor::patch_status(ctx.k8s.clone(), self, ActorState::building()).await?;
        Ok(())
    }

    async fn build(&self, ctx: Arc<Context>) -> Result<()> {
        let recorder = ctx.recorder(self.reference());

        // TODO: Return if the image already exists

        match job::exists(ctx.k8s.clone(), self).await? {
            true => {
                // Build job already exists, update it if there are new changes
                trace(
                    &recorder,
                    format!(
                        "Build job {} already exists, update it if there are new changes",
                        self.spec.build_name()
                    ),
                )
                .await?;
                job::update(ctx.k8s.clone(), self).await?;
            }
            false => {
                // Create a new image
                trace(
                    &recorder,
                    format!("Create new build Job: {}", self.spec.build_name()),
                )
                .await?;
                job::create(ctx.k8s.clone(), self).await?;
            }
        }

        trace(&recorder, "The images builded, Running").await?;
        let condition = ActorState::running(true, "AutoRun", None);
        actor::patch_status(ctx.k8s.clone(), self, condition).await?;

        Ok(())
    }

    // #[allow(dead_code)]
    // async fn build_with_kpack(&self, ctx: &Arc<Context>, recorder: &Recorder) -> Result<()> {
    //     match image::exists(ctx.k8s.clone(), self).await? {
    //         true => {
    //             // Image already exists, update it if there are new changes
    //             trace(
    //                 recorder,
    //                 format!(
    //                     "Image {} already exists, update it if there are new changes",
    //                     self.spec.build_name()
    //                 ),
    //             )
    //             .await?;
    //             image::update(ctx.k8s.clone(), self).await?;
    //         }
    //         false => {
    //             // Create a new image
    //             trace(
    //                 recorder,
    //                 format!("Create new image: {}", self.spec.build_name()),
    //             )
    //             .await?;
    //             image::create(ctx.k8s.clone(), self).await?;
    //         }
    //     }

    //     Ok(())
    // }

    async fn run(&self, ctx: Arc<Context>) -> Result<()> {
        let recorder = ctx.recorder(self.reference());
        trace(
            &recorder,
            format!(
                "Try to deploying the resources for Actor {}",
                self.name_any()
            ),
        )
        .await?;

        match deployment::exists(ctx.k8s.clone(), self).await? {
            true => {
                // Deployment already exists, update it if there are new changes
                trace(
                    &recorder,
                    format!(
                        "Deployment {} already exists, update it if there are new changes",
                        self.name_any()
                    ),
                )
                .await?;
                deployment::update(ctx.k8s.clone(), self).await?;
            }
            false => {
                // Create a new Deployment
                trace(
                    &recorder,
                    format!("Create new Deployment: {}", self.name_any()),
                )
                .await?;
                deployment::create(ctx.k8s.clone(), self).await?;
            }
        }

        if self.spec.service_ports().is_some() {
            match service::exists(ctx.k8s.clone(), self).await? {
                true => {
                    // Service already exists, update it if there are new changes
                    trace(
                        &recorder,
                        format!(
                            "Service {} already exists, update it if there are new changes",
                            self.name_any()
                        ),
                    )
                    .await?;
                    service::update(ctx.k8s.clone(), self).await?;
                }
                false => {
                    // Create a new Service
                    trace(
                        &recorder,
                        format!("Create new Service: {}", self.name_any()),
                    )
                    .await?;
                    service::create(ctx.k8s.clone(), self).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn cleanup(&self, ctx: Arc<Context>) -> Result<Action> {
        let namespace = self
            .namespace()
            .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
        let api: Api<Namespace> = Api::all(ctx.k8s.clone());

        let ns = api
            .get(namespace.as_str())
            .await
            .map_err(Error::KubeError)?;
        if let Some(status) = ns.status {
            if status.phase == Some("Terminating".into()) {
                return Ok(Action::await_change());
            }
        }

        let recorder = ctx.recorder(self.reference());
        trace(&recorder, format!("Delete Actor `{}`", self.name_any())).await?;

        Ok(Action::await_change())
    }

    fn reference(&self) -> ObjectReference {
        self.object_ref(&())
    }
}
