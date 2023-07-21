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

use std::collections::HashMap;

use clap::Parser;
use config::Config;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::metadata::LevelFilter;
use tracing::{debug, error};
use tracing_subscriber::EnvFilter;

pub mod config;

#[derive(Serialize, Deserialize)]
pub struct Synchronization {
    pub kind: String,
    pub paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Vec<u8>>,
}

#[tokio::main]
async fn main() -> Result<(), async_nats::Error> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::TRACE.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // This returns an error if the `.env` file doesn't exist, but that's not what we want
    // since we're not going to use a `.env` file if we deploy this application.
    dotenv::dotenv().ok();

    // Parse our configuration from the environment.
    // This will exit with a help message if something is wrong.
    let config = Config::parse();
    debug!("the nats url: {:?}", config.nats_url);
    debug!("the subject: {:?}", config.subject);

    let client = async_nats::connect(&config.nats_url).await?;
    let mut subscriber = client.subscribe(config.subject).await?;

    while let Some(message) = subscriber.next().await {
        let synchronization = serde_json::from_slice::<Synchronization>(message.payload.as_ref());
        if let Err(err) = synchronization {
            error!("Received invalid message: {:?} with error: {:?}", message.payload, err);
            continue;
        }

        let req = synchronization.unwrap();
        debug!("Received valid message: kind={:?} paths={:?}", req.kind, req.paths);
    }

    Ok(())
}
