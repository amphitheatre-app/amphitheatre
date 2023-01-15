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

use kube::api::{Patch, PatchParams, PostParams};
use kube::core::{DynamicObject, GroupVersionKind};
use kube::discovery::ApiResource;
use kube::{Api, Client, ResourceExt};
use serde_json::{from_value, json};

use super::crds::ActorSpec;
use super::error::{Error, Result};

pub async fn exists(client: Client, namespace: String, name: String) -> Result<bool> {
    let api: Api<DynamicObject> = Api::namespaced_with(client, namespace.as_str(), &api_resource());

    Ok(api
        .get_opt(&name)
        .await
        .map_err(Error::KubeError)?
        .is_some())
}

pub async fn create(client: Client, namespace: String, spec: &ActorSpec) -> Result<DynamicObject> {
    let api: Api<DynamicObject> = Api::namespaced_with(client, namespace.as_str(), &api_resource());

    let resource = new(spec)?;
    tracing::debug!("The image resource:\n {:#?}\n", resource);

    let image = api
        .create(&PostParams::default(), &resource)
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Created image:\n {:#?}\n", image.name_any());

    Ok(image)
}

pub async fn update(client: Client, namespace: String, spec: &ActorSpec) -> Result<DynamicObject> {
    let api: Api<DynamicObject> = Api::namespaced_with(client, namespace.as_str(), &api_resource());

    let name = spec.image_name();
    let mut image = api.get(&name).await.map_err(Error::KubeError)?;
    tracing::debug!("The image \"{}\" already exists:\n {:#?}\n", name, image);

    let resource = new(spec)?;

    if image.data.pointer("/spec") != resource.data.pointer("/spec") {
        tracing::debug!("The updated image resource:\n {:#?}\n", resource);

        image = api
            .patch(
                &name,
                &PatchParams::apply("amp-composer"),
                &Patch::Apply(&resource),
            )
            .await
            .map_err(Error::KubeError)?;

        tracing::info!("Updated image:\n {:#?}\n", image.name_any());
    }

    Ok(image)
}

#[inline]
fn api_resource() -> ApiResource {
    ApiResource::from_gvk(&GroupVersionKind::gvk("kpack.io", "v1alpha2", "Image"))
}

fn new(spec: &ActorSpec) -> Result<DynamicObject> {
    let data = from_value(json!({
        "spec": {
            "tag": spec.tag(),
            "serviceAccountName": "default",
            "builder": {
                "name": "amp-default-cluster-builder",
                "kind": "ClusterBuilder",
            },
            "source": {
                "git": {
                    "url": spec.repository,
                    "revision": spec.commit,
                },
                "subPath": spec.path.as_deref().unwrap_or_default(),
            }
        }
    }))
    .map_err(Error::SerializationError)?;

    Ok(DynamicObject::new(spec.image_name().as_str(), &api_resource()).data(data))
}
