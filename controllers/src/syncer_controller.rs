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

use futures::{future, StreamExt};
use k8s_openapi::api::core::v1::Namespace;
use kube::api::ListParams;
use kube::runtime::controller::Action;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::runtime::Controller;
use kube::Api;

use crate::context::Context;
use crate::error::{Error, Result};

pub async fn new(ctx: &Arc<Context>) {
    let api = Api::<Namespace>::all(ctx.k8s.clone());
    let params = ListParams::default().labels("syncer.amphitheatre.app/sync=true");

    Controller::new(api, params)
        .run(reconcile, error_policy, ctx.clone())
        .for_each(|_| future::ready(()))
        .await
}

/// The reconciler that will be called when either object change
pub async fn reconcile(ns: Arc<Namespace>, ctx: Arc<Context>) -> Result<Action> {
    let api: Api<Namespace> = Api::all(ctx.k8s.clone());

    let finalizer_name = "syncer.amphitheatre.app/finalizer";
    finalizer(&api, finalizer_name, ns, |event| async {
        match event {
            FinalizerEvent::Apply(ns) => apply(&ns, &ctx).await,
            FinalizerEvent::Cleanup(ns) => cleanup(&ns, &ctx).await,
        }
    })
    .await
    .map_err(|e| Error::FinalizerError(Box::new(e)))
}
/// an error handler that will be called when the reconciler fails with access to both the
/// object that caused the failure and the actual error
pub fn error_policy(_ns: Arc<Namespace>, error: &Error, _ctx: Arc<Context>) -> Action {
    tracing::error!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(60))
}

async fn apply(_ns: &Namespace, _ctx: &Arc<Context>) -> Result<Action> {
    Ok(Action::await_change())
}

pub async fn cleanup(_ns: &Namespace, _ctx: &Arc<Context>) -> Result<Action> {
    Ok(Action::await_change())
}
