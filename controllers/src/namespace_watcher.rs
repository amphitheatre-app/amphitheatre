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

use amp_resources::credential;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Namespace;
use kube::api::ListParams;
use kube::runtime::{watcher, WatchStreamExt};
use kube::{Api, ResourceExt};
use tracing::{error, info};

use crate::context::Context;

pub async fn new(ctx: &Arc<Context>) {
    let api = Api::<Namespace>::all(ctx.k8s.clone());
    let params = ListParams::default().labels("syncer.amphitheatre.app/sync=true");
    let mut obs = watcher(api, params).applied_objects().boxed();

    loop {
        let namespace = obs.try_next().await;

        match namespace {
            Ok(Some(ns)) => {
                if let Err(err) = handle(ctx, &ns).await {
                    error!("Handle namespace failed: {}", err.to_string());
                }
            }
            Ok(None) => continue,
            Err(err) => {
                error!("Resolve namespace stream failed: {}", err.to_string());
                continue;
            }
        }
    }
}

// This function lets the app handle an added namespace from k8s.
async fn handle(ctx: &Arc<Context>, ns: &Namespace) -> anyhow::Result<()> {
    info!("Handle an added namespace from k8s: {:#?}", ns.name_any());

    // Inject dependent credentials for this namespace.
    let configuration = ctx.configuration.read().await;
    credential::sync(&ctx.k8s, &ns.name_any(), &configuration).await?;

    Ok(())
}
