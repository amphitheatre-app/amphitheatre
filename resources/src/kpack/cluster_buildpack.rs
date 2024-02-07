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

use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::{DynamicObject, GroupVersionKind};
use kube::discovery::ApiResource;
use kube::{Api, Client, ResourceExt};
use serde_json::{from_value, json};

use tracing::{debug, info};

use crate::error::{Error, Result};
use crate::kpack::encode_name;

pub async fn exists(client: &Client, image: &str) -> Result<bool> {
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource());
    Ok(api.get_opt(&encode_name(image)).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, image: &str) -> Result<DynamicObject> {
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource());

    let resource = new(image)?;
    let buildpack = api.create(&PostParams::default(), &resource).await.map_err(Error::KubeError)?;
    info!("Created ClusterBuildpack: {}", buildpack.name_any());

    Ok(buildpack)
}

pub async fn update(client: &Client, image: &str) -> Result<DynamicObject> {
    let name = encode_name(image);
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource());

    let mut buildpack = api.get(&name).await.map_err(Error::KubeError)?;
    debug!("The ClusterBuildpack \"{}\" already exists", name);

    let resource = new(image)?;
    if buildpack.data.pointer("/spec") != resource.data.pointer("/spec") {
        debug!("The updating ClusterBuildpack resource:\n {:?}\n", resource);
        buildpack = api
            .patch(&name, &PatchParams::apply("amp-controllers").force(), &Patch::Apply(&resource))
            .await
            .map_err(Error::KubeError)?;

        info!("Updated ClusterBuildpack: {}", buildpack.name_any());
    }

    Ok(buildpack)
}

fn new(image: &str) -> Result<DynamicObject> {
    let resource = from_value(json!({
        "apiVersion": "kpack.io/v1alpha2",
        "kind": "ClusterBuildpack",
        "metadata": {
            "name": encode_name(image),
            "labels": {
                "app.kubernetes.io/managed-by": "Amphitheatre",
            },
        },
        "spec": {
            "serviceAccountRef": {
                "name": "amp-controllers", // @TODO: Use the specific service account from configuration
                "namespace": "amp-system", // @TODO: Use the namespace from configuration
            },
            "image": image.to_string(),
        }
    }))
    .map_err(Error::SerializationError)?;

    Ok(resource)
}

pub async fn ready(client: &Client, image: &str) -> Result<bool> {
    let name = encode_name(image);
    debug!("Check if the ClusterBuildpack {} is ready", name);

    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource());

    if let Some(buildpack) = api.get_opt(&name).await.map_err(Error::KubeError)? {
        debug!("Found ClusterBuildpack {}", buildpack.name_any());
        debug!("The ClusterBuildpack data is: {:?}", buildpack.data);

        if let Some(conditions) = buildpack.data.pointer("/status/conditions") {
            let conditions: Vec<Condition> =
                serde_json::from_value(json!(conditions)).map_err(Error::SerializationError)?;
            return Ok(conditions.iter().any(|condition| condition.type_ == "Ready" && condition.status == "True"));
        }
    }

    debug!("Not found ClusterBuildpack {}", image);
    Ok(false)
}

#[inline]
fn api_resource() -> ApiResource {
    ApiResource::from_gvk(&GroupVersionKind::gvk("kpack.io", "v1alpha2", "ClusterBuildpack"))
}
