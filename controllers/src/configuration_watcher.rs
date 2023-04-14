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

use amp_common::config::Configuration;
use amp_resources::credential;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::ListParams;
use kube::runtime::{watcher, WatchStreamExt};
use kube::Api;
use tracing::{debug, error, info};

use crate::context::Context;

pub async fn new(ctx: &Arc<Context>) {
    let namespace = ctx.config.namespace.clone();
    debug!("namespace = {}", namespace);

    let api = Api::<ConfigMap>::namespaced(ctx.k8s.clone(), &namespace);

    let params = ListParams::default().fields("metadata.name=amp-configurations");
    let mut obs = watcher(api, params).applied_objects().boxed();

    loop {
        let config_map = obs.try_next().await;

        match config_map {
            Ok(Some(cm)) => {
                if let Err(err) = handle(ctx, &cm).await {
                    error!("Handle config map failed: {}", err.to_string());
                }
            }
            Ok(None) => continue,
            Err(err) => {
                error!("Resolve config config stream failed: {}", err.to_string());
                continue;
            }
        }
    }
}

// This function lets the app handle an added/modified configmap from k8s.
async fn handle(ctx: &Arc<Context>, cm: &ConfigMap) -> anyhow::Result<()> {
    info!("Handle an added/modified configmap from k8s: {:#?}", cm.data);

    if let Some(data) = &cm.data {
        if let Some(content) = data.get("confgiuration.toml") {
            let value: Configuration = toml::from_str(content)?;

            let mut configuration = ctx.configuration.write().await;
            *configuration = value;
            info!("The latest configuration has been successfully applied!");

            // Refresh the credentials under the amp platform's own namespace.
            let configuration = ctx.configuration.read().await;
            credential::sync(
                &ctx.k8s,
                &ctx.config.namespace,
                &ctx.config.service_account_name,
                &configuration,
            )
            .await?;
        }
    }

    Ok(())
}
