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
use kube::runtime::finalizer::{finalizer, Event};
use kube::{Api, Client, ResourceExt};

use super::error::{Error, Result};
use super::types::{Playbook, PLAYBOOK_RESOURCE_NAME};

pub struct Context {
    pub client: Client,
}

/// The reconciler that will be called when either object change
pub async fn reconcile(playbook: Arc<Playbook>, ctx: Arc<Context>) -> Result<Action> {
    let ns = playbook.namespace().unwrap(); // doc is namespace scoped
    let api: Api<Playbook> = Api::namespaced(ctx.client.clone(), &ns);

    tracing::info!("Reconciling Playbook \"{}\" in {}", playbook.name_any(), ns);
    finalizer(&api, PLAYBOOK_RESOURCE_NAME, playbook, |event| async {
        match event {
            Event::Apply(playbook) => playbook.reconcile(ctx.clone()).await,
            Event::Cleanup(playbook) => playbook.cleanup(ctx.clone()).await,
        }
    })
    .await
    .map_err(Error::FinalizerError)
    // Ok(Action::requeue(Duration::from_secs(300)))
}
/// an error handler that will be called when the reconciler fails with access to both the
/// object that caused the failure and the actual error
pub fn error_policy(playbook: Arc<Playbook>, error: &Error, ctx: Arc<Context>) -> Action {
    tracing::warn!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(60))
}

impl Playbook {
    pub async fn reconcile(&self, ctx: Arc<Context>) -> Result<Action, kube::Error> {
        // If no events were received, check back every 5 minutes
        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    pub async fn cleanup(&self, _ctx: Arc<Context>) -> Result<Action, kube::Error> {
        // todo add some deletion event logging, db clean up, etc.?
        Ok(Action::await_change())
    }
}
