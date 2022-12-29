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

use std::sync::Arc;
use std::time::Duration;

use kube::runtime::controller::Action;
use kube::runtime::events::{Event, EventType, Recorder};
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::{Api, Client, Resource, ResourceExt};

use super::error::{Error, Result};
use super::resource;
use super::types::{Playbook, PLAYBOOK_RESOURCE_NAME};

pub struct Ctx {
    /// Kubernetes client
    pub client: Client,
}

impl Ctx {
    fn recorder(&self, playbook: &Playbook) -> Recorder {
        Recorder::new(
            self.client.clone(),
            "amphitheatre-composer".into(),
            playbook.object_ref(&()),
        )
    }
}

/// The reconciler that will be called when either object change
pub async fn reconcile(playbook: Arc<Playbook>, ctx: Arc<Ctx>) -> Result<Action> {
    let ns = playbook.namespace().unwrap(); // doc is namespace scoped
    let api: Api<Playbook> = Api::namespaced(ctx.client.clone(), &ns);

    tracing::info!("Reconciling Playbook \"{}\" in {}", playbook.name_any(), ns);
    if playbook.spec.actors.is_empty() {
        return Err(Error::EmptyActorsError);
    }

    finalizer(&api, PLAYBOOK_RESOURCE_NAME, playbook, |event| async {
        match event {
            FinalizerEvent::Apply(playbook) => playbook.reconcile(ctx.clone()).await,
            FinalizerEvent::Cleanup(playbook) => playbook.cleanup(ctx.clone()).await,
        }
    })
    .await
    .map_err(|e| Error::FinalizerError(Box::new(e)))
    // Ok(Action::requeue(Duration::from_secs(300)))
}
/// an error handler that will be called when the reconciler fails with access to both the
/// object that caused the failure and the actual error
pub fn error_policy(playbook: Arc<Playbook>, error: &Error, ctx: Arc<Ctx>) -> Action {
    tracing::warn!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(60))
}

impl Playbook {
    pub async fn reconcile(&self, ctx: Arc<Ctx>) -> Result<Action> {
        if self.status == Some(super::types::PlaybookStatus::Solved) {
            for actor in &self.spec.actors {
                resource::build(ctx.client.clone(), self, actor)
                    .await
                    .unwrap();
            }
        }

        // If no events were received, check back every 5 minutes
        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    pub async fn cleanup(&self, ctx: Arc<Ctx>) -> Result<Action> {
        // todo add some deletion event logging, db clean up, etc.?
        let recorder = ctx.recorder(self);
        // Doesn't have dependencies in this example case, so we just publish an event
        recorder
            .publish(Event {
                type_: EventType::Normal,
                reason: "DeletePlaybook".into(),
                note: Some(format!("Delete playbook `{}`", self.name_any())),
                action: "Reconciling".into(),
                secondary: None,
            })
            .await
            .map_err(Error::KubeError)?;
        Ok(Action::await_change())
    }
}
