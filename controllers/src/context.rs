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

use amp_common::config::Credentials;
use amp_resources::credential;
use async_nats::jetstream;
use tokio::sync::RwLock;

use crate::config::Config;

/// The core type through which handler functions can access common API state.
///
/// This can be accessed by adding a parameter `Extension<Context>` to a handler
///  function's  parameters.
///
/// It may not be a bad idea if you need your API to be more modular (turn routes
/// on and off, and disable any unused extension objects) but it's really up to a
/// judgement call.
pub struct Context {
    pub k8s: kube::Client,
    pub credentials: Arc<RwLock<Credentials>>,
    pub config: Arc<Config>,
    pub jetstream: Arc<jetstream::Context>,
}

impl Context {
    pub async fn new(config: Config) -> anyhow::Result<Context> {
        let k8s = kube::Client::try_default().await?;
        let credentials = credential::load(&k8s, &config.namespace).await?;
        let credentials = RwLock::new(credentials.unwrap_or_default());

        // Connect to NATS and create a JetStream instance.
        let client = async_nats::connect(&config.nats_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to NATS: {}, {}", &config.nats_url, e))?;
        let jetstream = jetstream::new(client);

        Ok(Context {
            k8s,
            credentials: Arc::new(credentials),
            config: Arc::new(config),
            jetstream: Arc::new(jetstream),
        })
    }
}
