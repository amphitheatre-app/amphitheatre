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

use kube::Client;

use crate::config::Config;
use crate::database::{self, Database};

/// The core type through which handler functions can access common API state.
///
/// This can be accessed by adding a parameter `Extension<Context>` to a handler
///  function's  parameters.
///
/// It may not be a bad idea if you need your API to be more modular (turn routes
/// on and off, and disable any unused extension objects) but it's really up to a
/// judgement call.
pub struct Context {
    pub config: Config,
    pub db: Database,
    pub k8s: Client,
}

impl Context {
    pub async fn new(config: Config) -> anyhow::Result<Context> {
        let dsn = format!(
            "mysql://{}:{}@{}:{}/{}",
            config.database_username,
            config.database_password,
            config.database_host,
            config.database_port,
            config.database_name
        );

        Ok(Context {
            config,
            db: database::new(dsn).await?,
            k8s: Client::try_default().await?,
        })
    }
}
