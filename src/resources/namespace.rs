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

use std::collections::BTreeMap;

use k8s_openapi::api::core::v1::Namespace;
use kube::api::{DeleteParams, Patch, PatchParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, ResourceExt};
use serde_json::to_string;

use super::error::Result;
use crate::resources::error::Error;

pub async fn create(client: Client, name: &String) -> Result<Namespace> {
    let api: Api<Namespace> = Api::all(client);
    let params = PatchParams::apply("amp-composer");

    let resource = Namespace {
        metadata: ObjectMeta {
            name: Some(name.to_owned()),
            // owner_references: todo!(),
            labels: Some(BTreeMap::from([(
                "app.kubernetes.io/managed-by".into(),
                "Amphitheatre".into(),
            )])),
            ..ObjectMeta::default()
        },
        ..Namespace::default()
    };
    tracing::debug!("The namespace resource:\n {:#?}\n", to_string(&resource));

    let namespace = api
        .patch(name, &params, &Patch::Apply(&resource))
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Added Namespace {:?}", namespace.name_any());
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
