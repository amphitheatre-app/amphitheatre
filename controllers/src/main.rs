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

use amp_crds::actor::Actor;
use amp_crds::playbook::Playbook;
use clap::Parser;
use futures::{future, StreamExt};
use kube::api::ListParams;
use kube::runtime::Controller;
use kube::Api;
use tracing::Level;

mod config;
mod context;

use crate::config::Config;
use crate::context::Context;

pub mod actor_controller;
pub mod playbook_controller;

/// Initialize the controller and shared state (given the crd is installed)
pub async fn run(ctx: Arc<Context>) {
    let playbook = Api::<Playbook>::all(ctx.k8s.clone());
    let actor = Api::<Actor>::all(ctx.k8s.clone());

    // Ensure Playbook CRD is installed before loop-watching
    if let Err(e) = playbook.list(&ListParams::default().limit(1)).await {
        tracing::error!("Playbook CRD is not queryable; {e:?}. Is the CRD installed?");
        tracing::info!("Installation: cargo run --bin crdgen | kubectl apply -f -");
        std::process::exit(1);
    }

    // Ensure Actor CRD is installed before loop-watching
    if let Err(e) = actor.list(&ListParams::default().limit(1)).await {
        tracing::error!("Actor CRD is not queryable; {e:?}. Is the CRD installed?");
        tracing::info!("Installation: cargo run --bin crdgen | kubectl apply -f -");
        std::process::exit(1);
    }

    // Create playbook controller
    let playbook_ctrl = Controller::new(playbook, ListParams::default())
        .run(
            playbook_controller::reconcile,
            playbook_controller::error_policy,
            ctx.clone(),
        )
        .for_each(|_| future::ready(()));

    // Create actor controller
    let actor_ctrl = Controller::new(actor, ListParams::default())
        .run(
            actor_controller::reconcile,
            actor_controller::error_policy,
            ctx.clone(),
        )
        .for_each(|_| future::ready(()));

    tokio::select! {
        _ = playbook_ctrl => tracing::warn!("playbook controller exited"),
        _ = actor_ctrl => tracing::warn!("actor controller exited"),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // This returns an error if the `.env` file doesn't exist, but that's not what we want
    // since we're not going to use a `.env` file if we deploy this application.
    dotenv::dotenv().ok();

    // Parse our configuration from the environment.
    // This will exit with a help message if something is wrong.
    let config = Config::parse();

    // Initialize the shared context.
    let ctx = Arc::new(Context::new(config).await?);
    run(ctx.clone()).await;

    Ok(())
}
