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

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::api::ListParams;
use kube::runtime::{watcher, WatchStreamExt};
use kube::Api;

use crate::context::Context;
use crate::error::Error::ResolveConfigMapStreamFailed;
use crate::error::Result;

pub async fn new(ctx: &Arc<Context>) -> Result<()> {
    let api = Api::<ConfigMap>::namespaced(ctx.k8s.clone(), "amp-system");
    let params = ListParams::default()
        .fields("metadata.name=amp-configurations")
        .timeout(10);

    let mut obs = watcher(api, params).applied_objects().boxed();
    while let Some(cm) = &obs.try_next().await.map_err(ResolveConfigMapStreamFailed)? {
        handle_config_map(ctx, cm)?;
    }

    Ok(())
}

// This function lets the app handle an added/modified configmap from k8s.
fn handle_config_map(_ctx: &Arc<Context>, cm: &ConfigMap) -> Result<()> {
    tracing::info!("ConfigMap {:#?}", cm.data);
    Ok(())
}
