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

use amphitheatre::app::{self, Context};
use amphitheatre::config::Config;
use amphitheatre::{composer, database};
use clap::Parser;
use kube::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // This returns an error if the `.env` file doesn't exist, but that's not what we want
    // since we're not going to use a `.env` file if we deploy this application.
    dotenv::dotenv().ok();

    // Parse our configuration from the environment.
    // This will exit with a help message if something is wrong.
    let config = Config::parse();

    let dsn = config.database_url.clone();
    let ctx = Arc::new(Context {
        config,
        db: database::new(dsn).await?,
        k8s: Client::try_default().await?,
    });

    composer::init(ctx.clone()).await;

    // Finally, we spin up our API.
    app::run(ctx).await;

    Ok(())
}
