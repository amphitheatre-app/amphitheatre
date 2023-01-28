// Copyright 2022 The Amphitheatre Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(unused_variables)]

use std::sync::Arc;

use amphitheatre::config::Config;
use amphitheatre::context::Context;
use amphitheatre::{app, composer};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // This returns an error if the `.env` file doesn't exist, but that's not what we want
    // since we're not going to use a `.env` file if we deploy this application.
    dotenv::dotenv().ok();

    // Parse our configuration from the environment.
    // This will exit with a help message if something is wrong.
    let config = Config::parse();

    // Initialize the shared context.
    let ctx = Arc::new(Context::new(config).await?);

    tokio::select! {
        _ = composer::run(ctx.clone()) => tracing::warn!("composer exited"),
        _ = app::run(ctx.clone()) => tracing::info!("server exited"),
    }

    Ok(())
}
