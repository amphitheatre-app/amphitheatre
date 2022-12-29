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

use futures::{future, StreamExt};
use kube::api::ListParams;
use kube::runtime::Controller;
use kube::Api;

use self::controller::{error_policy, reconcile, Ctx};
use self::types::Playbook;
use crate::app::Context;

pub mod controller;
pub mod error;
pub mod resource;
pub mod types;

/// Initialize the controller and shared state (given the crd is installed)
pub async fn run(ctx: Arc<Context>) {
    let api = Api::<Playbook>::all(ctx.k8s.clone());

    // Ensure CRD is installed before loop-watching
    if let Err(e) = api.list(&ListParams::default().limit(1)).await {
        tracing::error!("CRD is not queryable; {e:?}. Is the CRD installed?");
        tracing::info!("Installation: cargo run --bin crdgen | kubectl apply -f -");
        std::process::exit(1);
    }

    let context = Arc::new(Ctx {
        client: ctx.k8s.clone(),
    });

    Controller::new(api, ListParams::default())
        .run(reconcile, error_policy, context)
        .for_each(|_| future::ready(()))
        .await;
}
