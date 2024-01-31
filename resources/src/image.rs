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

use amp_common::resource::Actor;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::{DynamicObject, GroupVersionKind};
use kube::discovery::ApiResource;
use kube::{Api, Client, Resource, ResourceExt};
use serde_json::{from_value, json};
use tracing::{debug, info};

use crate::error::{Error, Result};

pub async fn exists(client: &Client, actor: &Actor) -> Result<bool> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace.as_str(), &api_resource());
    let name = format!("{}-builder", actor.spec.name);

    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, actor: &Actor) -> Result<DynamicObject> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace.as_str(), &api_resource());

    let resource = new(actor)?;
    debug!("The Image resource:\n {:?}\n", resource);

    let image = api.create(&PostParams::default(), &resource).await.map_err(Error::KubeError)?;

    info!("Created Image: {}", image.name_any());

    Ok(image)
}

pub async fn update(client: &Client, actor: &Actor) -> Result<DynamicObject> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace.as_str(), &api_resource());

    let name = format!("{}-builder", actor.spec.name);
    let mut image = api.get(&name).await.map_err(Error::KubeError)?;
    debug!("The Image \"{}\" already exists", name);

    let resource = new(actor)?;

    if image.data.pointer("/spec") != resource.data.pointer("/spec") {
        debug!("The updating Image resource:\n {:?}\n", resource);

        image = api
            .patch(&name, &PatchParams::apply("amp-controllers").force(), &Patch::Apply(&resource))
            .await
            .map_err(Error::KubeError)?;

        info!("Updated Image: {}", image.name_any());
    }

    Ok(image)
}

#[inline]
fn api_resource() -> ApiResource {
    ApiResource::from_gvk(&GroupVersionKind::gvk("kpack.io", "v1alpha2", "Image"))
}

fn new(actor: &Actor) -> Result<DynamicObject> {
    let name = format!("{}-builder", actor.spec.name);
    let source = actor.spec.source.as_ref().unwrap();

    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let resource = from_value(json!({
        "apiVersion": "kpack.io/v1alpha2",
        "kind": "Image",
        "metadata": {
            "name": name.clone(),
            "ownerReferences": vec![owner_reference],
            "labels": {
                "amphitheatre.app/character": name.clone(),
                "app.kubernetes.io/managed-by": "Amphitheatre",
            },
        },
        "spec": {
            "tag": actor.spec.image,
            "builder": {
                "name": "default",
                "kind": "ClusterBuilder",
            },
            "source": {
                "git": {
                    "url": source.repo,
                    "revision": source.rev(),
                },
                "subPath": source.path.as_deref().unwrap_or_default(),
            }
        }
    }))
    .map_err(Error::SerializationError)?;

    Ok(resource)
}

pub async fn completed(client: &Client, actor: &Actor) -> Result<bool> {
    debug!("Check If the build image has not completed");

    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace.as_str(), &api_resource());
    let name = format!("{}-builder", actor.spec.name);

    if let Some(image) = api.get_opt(&name).await.map_err(Error::KubeError)? {
        debug!("Found Image {}", &name);
        debug!("The Image data is: {:?}", image.data);

        if let Some(conditions) = image.data.pointer("/status/conditions") {
            let conditions: Vec<Condition> =
                serde_json::from_value(json!(conditions)).map_err(Error::SerializationError)?;
            return Ok(conditions.iter().any(|condition| condition.type_ == "Ready" && condition.status == "True"));
        }
    }

    debug!("Not found Image {}", &name);
    Ok(false)
}
