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

use kube::runtime::controller::Action;
use kube::runtime::events::{Event, EventType};
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::{Api, Resource, ResourceExt};

use super::Ctx;
use crate::resources::actor;
use crate::resources::crds::{Actor, ActorState};
use crate::resources::error::{Error, Result};

/// The reconciler that will be called when either object change
pub async fn reconcile(actor: Arc<Actor>, ctx: Arc<Ctx>) -> Result<Action> {
    let ns = actor.namespace().unwrap(); // actor is namespace scoped
    let api: Api<Actor> = Api::namespaced(ctx.client.clone(), &ns);

    tracing::info!("Reconciling Actor \"{}\"", actor.name_any());

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
pub fn error_policy(actor: Arc<Actor>, error: &Error, ctx: Arc<Ctx>) -> Action {
    tracing::error!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(60))
}

impl Actor {
    pub async fn reconcile(&self, ctx: Arc<Ctx>) -> Result<Action> {
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
    }

    async fn init(&self, ctx: Arc<Ctx>) -> Result<()> {
        actor::patch_status(ctx.client.clone(), self, ActorState::building()).await?;
        Ok(())
    }

    async fn build(&self, ctx: Arc<Ctx>) -> Result<()> {
        actor::build(ctx.client.clone(), self).await?;
        Ok(())
    }

    async fn run(&self, ctx: Arc<Ctx>) -> Result<()> {
        actor::deploy(ctx.client.clone(), self).await?;
        Ok(())
    }

    pub async fn cleanup(&self, ctx: Arc<Ctx>) -> Result<Action> {
        // todo add some deletion event logging, db clean up, etc.?
        let recorder = ctx.recorder(self.object_ref(&()));
        // Doesn't have dependencies in this example case, so we just publish an event
        recorder
            .publish(Event {
                type_: EventType::Normal,
                reason: "DeleteActor".into(),
                note: Some(format!("Delete actor `{}`", self.name_any())),
                action: "Reconciling".into(),
                secondary: None,
            })
            .await
            .map_err(Error::KubeError)?;
        Ok(Action::await_change())
    }
}
