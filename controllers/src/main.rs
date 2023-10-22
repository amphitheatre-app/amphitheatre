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

#![allow(clippy::enum_variant_names)]
use std::sync::Arc;

use clap::Parser;
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

mod config;
mod context;
mod error;

use crate::config::Config;
use crate::context::Context;

mod actor_controller;
mod credentials_watcher;
mod namespace_watcher;
mod playbook_controller;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // This returns an error if the `.env` file doesn't exist, but that's not what we want
    // since we're not going to use a `.env` file if we deploy this application.
    dotenv::dotenv().ok();

    // Parse our configuration from the environment.
    // This will exit with a help message if something is wrong.
    // Then, initialize the shared context.
    let ctx = Arc::new(Context::new(Config::parse()).await?);

    // Creates the controllers and waits on multiple concurrent branches,
    // returning when **the first** branch completes and cancelling the remaining branches.
    tokio::select! {
        _ = playbook_controller::new(&ctx) => tracing::warn!("playbook controller exited"),
        _ = actor_controller::new(&ctx) => tracing::warn!("actor controller exited"),
        _ = credentials_watcher::new(&ctx) => tracing::warn!("credentials watcher exited"),
        _ = namespace_watcher::new(&ctx) => tracing::warn!("namespace watcher exited"),
    }

    Ok(())
}
