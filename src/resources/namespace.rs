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

use k8s_openapi::api::core::v1::Namespace;
use kube::api::{DeleteParams, PostParams};
use kube::{Api, Client};
use serde_json::{from_value, json};

use super::error::Result;
use crate::resources::error::Error;

pub async fn create(client: Client, name: &String) -> Result<Namespace> {
    let api: Api<Namespace> = Api::all(client);
    let params = PostParams::default();

    let mut namespace = from_value(json!({
        "apiVersion": "v1",
        "kind": "Namespace",
        "metadata": {
            "name": name,
            "labels": {
                "app.kubernetes.io/managed-by": "Amphitheatre"
            }
        }
    }))
    .map_err(Error::SerializationError)?;

    tracing::info!(
        "created namespace resource: {:#?}",
        serde_yaml::to_string(&namespace)
    );

    namespace = api
        .create(&params, &namespace)
        .await
        .map_err(Error::KubeError)?;

    Ok(namespace)
}

pub async fn delete(client: Client, name: String) -> Result<()> {
    let api: Api<Namespace> = Api::all(client);
    let params = DeleteParams::default();

    // Ignore delete error if not exists
    let _ = api.delete(&name, &params).await.map(|res| {
        res.map_left(|o| tracing::info!("Deleting namespace: {:?}", o.status))
            .map_right(|s| tracing::info!("Deleted namespace: {:?}", s));
    });

    Ok(())
}
