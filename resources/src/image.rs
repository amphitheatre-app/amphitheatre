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

use amp_common::schema::Actor;
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::{DynamicObject, GroupVersionKind};
use kube::discovery::ApiResource;
use kube::{Api, Client, Resource, ResourceExt};
use serde_json::{from_value, json};

use super::error::{Error, Result};

pub async fn exists(client: Client, actor: &Actor) -> Result<bool> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<DynamicObject> = Api::namespaced_with(client, namespace.as_str(), &api_resource());
    let name = actor.spec.build_name();

    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: Client, actor: &Actor) -> Result<DynamicObject> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<DynamicObject> = Api::namespaced_with(client, namespace.as_str(), &api_resource());

    let resource = new(actor)?;
    tracing::debug!("The image resource:\n {:#?}\n", resource);

    let image = api
        .create(&PostParams::default(), &resource)
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Created image: {}", image.name_any());

    Ok(image)
}

pub async fn update(client: Client, actor: &Actor) -> Result<DynamicObject> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<DynamicObject> = Api::namespaced_with(client, namespace.as_str(), &api_resource());

    let name = actor.spec.build_name();
    let mut image = api.get(&name).await.map_err(Error::KubeError)?;
    tracing::debug!("The image \"{}\" already exists:\n {:#?}\n", name, image);

    let resource = new(actor)?;

    if image.data.pointer("/spec") != resource.data.pointer("/spec") {
        tracing::debug!("The updating image resource:\n {:#?}\n", resource);

        image = api
            .patch(
                &name,
                &PatchParams::apply("amp-composer").force(),
                &Patch::Apply(&resource),
            )
            .await
            .map_err(Error::KubeError)?;

        tracing::info!("Updated image: {}", image.name_any());
    }

    Ok(image)
}

#[inline]
fn api_resource() -> ApiResource {
    ApiResource::from_gvk(&GroupVersionKind::gvk("kpack.io", "v1alpha2", "Image"))
}

fn new(actor: &Actor) -> Result<DynamicObject> {
    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let resource = from_value(json!({
        "apiVersion": "kpack.io/v1alpha2",
        "kind": "Image",
        "metadata": {
            "name": actor.spec.build_name(),
            "ownerReferences": vec![owner_reference]
        },
        "spec": {
            "tag": actor.spec.docker_tag(),
            "serviceAccountName": "default",
            "builder": {
                "name": actor.spec.builder(),
                "kind": "ClusterBuilder",
            },
            "source": {
                "git": {
                    "url": actor.spec.source.repo,
                    "revision": actor.spec.source.rev,
                },
                "subPath": actor.spec.source.path.as_deref().unwrap_or_default(),
            }
        }
    }))
    .map_err(Error::SerializationError)?;

    Ok(resource)
}
