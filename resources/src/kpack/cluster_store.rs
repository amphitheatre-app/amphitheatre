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

use super::BuildExt;
use crate::error::{Error, Result};
use amp_common::resource::Actor;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::{DynamicObject, GroupVersionKind};
use kube::discovery::ApiResource;
use kube::{Api, Client, ResourceExt};
use serde_json::{from_value, json};
use tracing::{debug, info};

pub async fn exists(client: &Client, actor: &Actor) -> Result<bool> {
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource());
    let name = actor.spec.character.store_name();

    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn get(client: &Client, actor: &Actor) -> Result<DynamicObject> {
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource());
    let name = actor.spec.character.store_name();

    api.get(&name).await.map_err(Error::KubeError)
}

pub async fn create(client: &Client, actor: &Actor) -> Result<DynamicObject> {
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource());

    let resource = new(actor).await?;
    let builder = api.create(&PostParams::default(), &resource).await.map_err(Error::KubeError)?;
    info!("Created ClusterStore: {}", builder.name_any());

    Ok(builder)
}

pub async fn update(client: &Client, actor: &Actor) -> Result<DynamicObject> {
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource());

    let name = actor.spec.character.store_name();
    let mut builder = api.get(&name).await.map_err(Error::KubeError)?;
    debug!("The ClusterStore \"{}\" already exists", name);

    let resource = new(actor).await?;
    if builder.data.pointer("/spec") != resource.data.pointer("/spec") {
        debug!("The updating ClusterStore resource:\n {:?}\n", resource);
        builder = api
            .patch(&name, &PatchParams::apply("amp-controllers").force(), &Patch::Apply(&resource))
            .await
            .map_err(Error::KubeError)?;

        info!("Updated ClusterStore: {}", builder.name_any());
    }

    Ok(builder)
}

#[inline]
fn api_resource() -> ApiResource {
    ApiResource::from_gvk(&GroupVersionKind::gvk("kpack.io", "v1alpha2", "ClusterStore"))
}

async fn new(actor: &Actor) -> Result<DynamicObject> {
    let name = actor.spec.character.store_name();
    let resource = from_value(json!({
        "apiVersion": "kpack.io/v1alpha2",
        "kind": "ClusterStore",
        "metadata": {
            "name": name.clone(),
            "labels": {
                "app.kubernetes.io/managed-by": "Amphitheatre",
            },
        },
        "spec": {
            "serviceAccountRef": {
                "name": "amp-controllers", // @TODO: Use the specific service account from configuration
                "namespace": "amp-system", // @TODO: Use the namespace from configuration
            },
            "sources": [{
                "image": actor.spec.character.store_image(),
            }],
        }
    }))
    .map_err(Error::SerializationError)?;

    Ok(resource)
}

pub async fn ready(client: &Client, actor: &Actor) -> Result<bool> {
    let name = actor.spec.character.store_name();
    debug!("Check if the ClusterStore {} is ready", name);

    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource());

    if let Some(store) = api.get_opt(&name).await.map_err(Error::KubeError)? {
        debug!("Found ClusterStore {}", &name);

        if let Some(conditions) = store.data.pointer("/status/conditions") {
            let conditions: Vec<Condition> =
                serde_json::from_value(json!(conditions)).map_err(Error::SerializationError)?;
            return Ok(conditions.iter().any(|condition| condition.type_ == "Ready" && condition.status == "True"));
        }
    }

    debug!("Not found ClusterStore {}", &name);
    Ok(false)
}
