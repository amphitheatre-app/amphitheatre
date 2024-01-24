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

use std::sync::Arc;
use std::time::Duration;

use amp_common::resource::Playbook;

use amp_workflow::Workflow;
use futures::{future, StreamExt};
use kube::api::ListParams;
use kube::runtime::controller::Action;
use kube::runtime::finalizer::{finalizer, Event};
use kube::runtime::{watcher, Controller};
use kube::{Api, ResourceExt};
use tracing::{error, info};

use crate::context::Context;
use crate::errors::{Error, Result};

const FINALIZER_NAME: &str = "playbooks.amphitheatre.app/finalizer";

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
    let api: Api<Playbook> = Api::all(ctx.k8s.clone());

    // Reconcile the playbook custom resource.
    finalizer(&api, FINALIZER_NAME, playbook.clone(), |event| async {
        let mut workflow = Workflow::new(
            amp_workflow::Context {
                k8s: Arc::new(ctx.k8s.clone()),
                jetstream: ctx.jetstream.clone(),
                credentials: ctx.credentials.clone(),
                object: playbook.clone(),
            },
            Box::new(amp_workflow::playbook::InitialState),
        );

        match event {
            Event::Apply(playbook) => {
                info!("Apply playbook {}", playbook.name_any());
                workflow.set_context(playbook.clone());
            }
            Event::Cleanup(playbook) => {
                info!("Cleanup playbook {}", playbook.name_any());
                workflow.set_context(playbook.clone());
                workflow.transition(Box::new(amp_workflow::playbook::CleanupState));
            }
        };

        // Runs the workflow until there is no next state
        workflow.run().await;

        Ok(Action::await_change())
    })
    .await
    .map_err(|e| Error::FinalizerError(Box::new(e)))
}

/// an error handler that will be called when the reconciler fails with access to both the
/// object that caused the failure and the actual error
pub fn error_policy(_: Arc<Playbook>, error: &Error, _: Arc<Context>) -> Action {
    error!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(60))
}
