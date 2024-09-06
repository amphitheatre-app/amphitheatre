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

use amp_common::resource::Playbook;
use amp_resources::playbook::delete;
use chrono::{DateTime, Duration, TimeDelta, Utc};
use futures::{future, StreamExt};
use kube::Client;
use kube::{
    runtime::{reflector, watcher, WatchStreamExt},
    Api,
};
use tracing::{error, info, warn};

use crate::context::Context;

/// The strategy is to evaluate the execution status of the playbook.
enum Strategy {
    /// Handling the expiration of the playbook.
    Expired,

    /// Handling the retention of the playbook.
    Remain(TimeDelta)
}

// Implement the From trait for Strategy.
impl From<DateTime<Utc>> for Strategy {
    fn from(value: DateTime<Utc>) -> Self {
        let now = Utc::now();
        if value < now {
            Self::Expired
        } else {
            Self::Remain(value - now)
        }
    }
}

pub async fn new(ctx: &Arc<Context>) {
    let client = ctx.k8s.clone();
    let api = Api::<Playbook>::all(client.clone());
    let config = watcher::Config::default();
    let (reader, writer) = reflector::store();
    let rf = reflector(writer, watcher(api, config));

    tokio::spawn(async move {
        if let Err(e) = reader.wait_until_ready().await {
            error!("Failed to wait until ready: {:?}", e);
            return;
        }
        info!("Timeout controller is running...");
        loop {
            for p in reader.state() {
                if let Err(err) = handle(p.as_ref(), &client).await {
                    error!("Delete playbook failed: {}", err.to_string());
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5 * 60)).await;
        }
    });

    rf.applied_objects()
        .for_each(|_| future::ready(()))
        .await;
    
}

async fn handle(playbook: &Playbook, client: &Client) -> anyhow::Result<()> {
    if let Some(ttl) = playbook
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get("ttl"))
        .and_then(|ttl_str| ttl_str.parse::<i64>().ok())
    {
        if let Some(timestamp) = playbook.metadata.creation_timestamp.as_ref() {
            let expiration_time = timestamp.0 + Duration::seconds(ttl);
            let stratege = Strategy::from(expiration_time);
            match stratege {
                Strategy::Expired => {
                    if let Some(name) = &playbook.metadata.name {
                        delete(client, name).await?;
                    }
                },
                Strategy::Remain(time) => {
                    if time == Duration::days(3)  {
                        send_message().await;
                    }
                },
            }
        }
    }
    Ok(())
}

async fn send_message() {
    warn!("Email sending functionality is not implemented yet");
}
