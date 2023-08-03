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

use amp_common::config::Credentials;
use amp_resources::credential;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Secret;
use kube::runtime::{watcher, WatchStreamExt};
use kube::{Api, ResourceExt};
use tracing::{debug, error, info};

use crate::context::Context;

pub async fn new(ctx: &Arc<Context>) {
    let namespace = ctx.config.namespace.clone();
    debug!("namespace = {}", namespace);

    let api = Api::<Secret>::namespaced(ctx.k8s.clone(), &namespace);
    let config = watcher::Config::default().fields("metadata.name=amp-credentials");
    let mut obs = watcher(api, config).applied_objects().boxed();

    loop {
        let secret = obs.try_next().await;
        match secret {
            Ok(Some(secret)) => {
                if let Err(err) = handle(ctx, &secret).await {
                    error!("Handle secret failed: {}", err.to_string());
                }
            }
            Ok(None) => continue,
            Err(err) => {
                error!("Resolve secret stream failed: {}", err.to_string());
                continue;
            }
        }
    }
}

// This function lets the app handle an added/modified secret from k8s.
async fn handle(ctx: &Arc<Context>, secret: &Secret) -> anyhow::Result<()> {
    info!("Handle an added/modified secret from k8s: {}", secret.name_any());

    if let Some(data) = &secret.data {
        if let Some(content) = data.get("credentials") {
            let content = std::str::from_utf8(&content.0)?;
            debug!("The credentials is: {:?}", content);

            let value: Credentials = toml::from_str(content)?;
            let mut credentials = ctx.credentials.write().await;
            *credentials = value;

            // Refresh the credentials under the amp platform's own namespace.
            debug!("Refresh the credentials under the amp platform's own namespace.");
            credential::sync(
                &ctx.k8s,
                &ctx.config.namespace,
                &ctx.config.service_account_name,
                &credentials,
            )
            .await?;

            info!("The latest credentials has been successfully applied!");
        }
    }

    Ok(())
}
