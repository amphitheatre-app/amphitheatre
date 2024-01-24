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

use amp_common::resource::Actor;
use amp_resources::deployer::Deployer;
use futures::{future, StreamExt};
use k8s_openapi::api::core::v1::Namespace;
use kube::api::ListParams;
use kube::runtime::controller::Action;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::runtime::{watcher, Controller};
use kube::{Api, ResourceExt};
use tracing::{error, info};

use crate::context::Context;
use crate::errors::{Error, Result};

pub async fn new(ctx: &Arc<Context>) {
    let api = Api::<Actor>::all(ctx.k8s.clone());

    // Ensure Actor CRD is installed before loop-watching
    if let Err(e) = api.list(&ListParams::default().limit(1)).await {
        error!("Actor CRD is not queryable; {e:?}. Is the CRD installed?");
        info!("Installation: amp-crdgen | kubectl apply -f -");
        std::process::exit(1);
    }

    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx.clone())
        .for_each(|_| future::ready(()))
        .await
}

/// The reconciler that will be called when either object change
pub async fn reconcile(actor: Arc<Actor>, ctx: Arc<Context>) -> Result<Action> {
    info!("Reconciling Actor \"{}\"", actor.name_any());

    let ns = actor.namespace().unwrap(); // actor is namespace scoped
    let api: Api<Actor> = Api::namespaced(ctx.k8s.clone(), &ns);

    // Reconcile the actor custom resource.
    let finalizer_name = "actors.amphitheatre.app/finalizer";
    finalizer(&api, finalizer_name, actor, |event| async {
        match event {
            FinalizerEvent::Apply(actor) => apply(&actor, &ctx).await,
            FinalizerEvent::Cleanup(actor) => cleanup(&actor, &ctx).await,
        }
    })
    .await
    .map_err(|e| Error::FinalizerError(Box::new(e)))
}
/// an error handler that will be called when the reconciler fails with access to both the
/// object that caused the failure and the actual error
pub fn error_policy(_actor: Arc<Actor>, error: &Error, _ctx: Arc<Context>) -> Action {
    error!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(60))
}

async fn apply(actor: &Actor, ctx: &Arc<Context>) -> Result<Action> {
    info!("Try to deploying the resources for Actor {}", actor.name_any());

    let credentials = ctx.credentials.read().await;
    let mut deployer = Deployer::new(ctx.k8s.clone(), &credentials, actor);
    deployer.run().await.map_err(Error::DeployError)?;

    Ok(Action::await_change())
}

pub async fn cleanup(actor: &Actor, ctx: &Arc<Context>) -> Result<Action> {
    let namespace = actor.namespace().unwrap();
    let api: Api<Namespace> = Api::all(ctx.k8s.clone());

    let ns = api.get(namespace.as_str()).await.map_err(Error::KubeError)?;
    if let Some(status) = ns.status {
        if status.phase == Some("Terminating".into()) {
            return Ok(Action::await_change());
        }
    }

    info!("Delete Actor `{}`", actor.name_any());

    Ok(Action::await_change())
}
