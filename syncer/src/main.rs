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

use std::path::Path;

use amp_common::sync::EventKinds::*;
use amp_common::sync::Synchronization;
use async_nats::jetstream::consumer::{pull, PullConsumer};
use async_nats::jetstream::{self, stream};
use clap::Parser;
use config::Config;
use futures::StreamExt;
use tracing::metadata::LevelFilter;
use tracing::{debug, error, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

mod config;
mod handle;

#[tokio::main]
async fn main() -> Result<(), async_nats::Error> {
    // Enable tracing.
    tracing_subscriber::registry()
        .with(EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env_lossy())
        .with(tracing_subscriber::fmt::layer().without_time().with_file(false).with_target(false))
        .init();

    // This returns an error if the `.env` file doesn't exist, but that's not what we want
    // since we're not going to use a `.env` file if we deploy this application.
    dotenv::dotenv().ok();

    // Parse our configuration from the environment.
    // This will exit with a help message if something is wrong.
    let config = Config::parse();
    debug!("Configuration: {:?}", config);

    // initialize some variables
    let workspace = Path::new(&config.workspace);

    debug!("Connecting to NATS server: {}", config.nats_url);
    let consumer = connect(&config).await?;

    // Consume messages from the consumer
    let mut messages = consumer.messages().await?;
    while let Some(Ok(message)) = messages.next().await {
        let synchronization = serde_json::from_slice(message.payload.as_ref());
        if let Err(err) = synchronization {
            error!("Received invalid message: {:?} with error: {:?}", message.payload, err);
            continue;
        }

        let req: Synchronization = synchronization.unwrap();
        debug!("Received valid message: kind={:?} paths={:?}", req.kind, req.paths);

        // Handle the message
        if let Err(err) = match req.kind {
            Create => handle::create(workspace, &req),
            Modify => handle::modify(workspace, &req),
            Rename => handle::rename(workspace, &req),
            Remove => handle::remove(workspace, &req),
            Overwrite => handle::overwrite(workspace, &req),
            Other => {
                warn!("Received other event, nothing to do!");
                Ok(())
            }
        } {
            // If we failed to handle the message, log the error and continue.
            // We don't want to crash the application because of a single message.
            // We can always retry later, but the next time retry,
            // the original intent may no longer be valid!!!
            error!("Failed to handle message: {}", err);
            continue;
        }
        // Acknowledge the message if we handled it successfully.
        if let Err(err) = message.ack().await {
            error!("Failed to acknowledge message: {:?}", err);
        }

        // If we're in once mode, exit after overwrite.
        if config.once && req.kind == Overwrite {
            info!("Finished syncing, exiting...");
            std::process::exit(0);
        }
    }

    Ok(())
}

/// Connect to NATS server and return a consumer.
async fn connect(config: &Config) -> Result<PullConsumer, async_nats::Error> {
    // Connect to NATS server and create a JetStream instance.
    let client = async_nats::connect(&config.nats_url).await?;
    let jetstream = jetstream::new(client);

    // get or create a stream and a consumer
    let subject = format!("{}.{}", config.playbook, config.actor);
    let name = "amp-syncer";
    let consumer = jetstream
        // First, on the `JetStream` instance, use method to create Stream.
        .get_or_create_stream(stream::Config {
            name: config.playbook.to_string(),
            subjects: vec![format!("{}.*", config.playbook)],
            ..Default::default()
        })
        .await?
        // Then, on that `Stream` use method to create Consumer and bind to it.
        .get_or_create_consumer(
            name,
            pull::Config {
                durable_name: Some(name.to_string()),
                filter_subject: subject.clone(),
                ..Default::default()
            },
        )
        .await?;
    info!("Subscribed to stream {} and subject: {}", config.playbook, subject);

    Ok(consumer)
}
