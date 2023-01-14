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

use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::{DynamicObject, GroupVersionKind};
use kube::discovery::ApiResource;
use kube::{Api, Client, ResourceExt};
use serde_json::{from_value, json};

use super::crds::{Actor, ActorSpec};
use super::deployment;
use super::error::{Error, Result};
use crate::resources::crds::ActorState;

pub async fn create(client: Client, namespace: String, spec: ActorSpec) -> Result<Actor> {
    let api: Api<Actor> = Api::namespaced(client.clone(), namespace.as_str());
    let params = PostParams::default();

    let mut actor = Actor::new(&spec.name.clone(), spec);
    tracing::info!("{:#?}", serde_yaml::to_string(&actor));

    actor = api
        .create(&params, &actor)
        .await
        .map_err(Error::KubeError)?;

    // Patch this actor as initial Pending status
    patch_status(client.clone(), &actor, ActorState::pending()).await?;
    Ok(actor)
}

pub async fn build(client: Client, actor: &Actor) -> Result<()> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;

    let gvk = GroupVersionKind::gvk("kpack.io", "v1alpha2", "Image");
    let ar = ApiResource::from_gvk(&gvk);
    let api: Api<DynamicObject> = Api::namespaced_with(client, namespace.as_str(), &ar);

    let params = PostParams::default();
    let resource = from_value(json!({
        "apiVersion": "kpack.io/v1alpha2",
        "kind": "Image",
        "metadata": {
            "name": format!("{}-{}", actor.spec.name, actor.spec.commit),
        },
        "spec": {
            "tag": format!("harbor.amp-system.svc.cluster.local/library/{}:{}", actor.spec.image, actor.spec.commit),
            "serviceAccountName": "default",
            "builder": {
                "name": "amp-default-cluster-builder",
                "kind": "ClusterBuilder",
            },
            "source": {
                "git": {
                    "url": actor.spec.repository,
                    "revision": actor.spec.commit,
                },
                "subPath": actor.spec.path,
            }
        }
    }))
    .map_err(Error::SerializationError)?;

    tracing::info!(
        "created image resource: {:#?}",
        serde_yaml::to_string(&resource)
    );
    api.create(&params, &resource)
        .await
        .map_err(Error::KubeError)?;

    Ok(())
}

pub async fn deploy(client: Client, actor: &Actor) -> Result<()> {
    // Create Deployment resource for this actor
    deployment::create(client, actor).await?;

    // TODO: Create Service resource if needed.

    Ok(())
}

pub async fn patch_status(client: Client, actor: &Actor, condition: Condition) -> Result<()> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;

    let api: Api<Actor> = Api::namespaced(client, &namespace);

    let status = json!({ "status": { "conditions": vec![condition] }});
    let actor = api
        .patch_status(
            actor.name_any().as_str(),
            &PatchParams::default(),
            &Patch::Merge(&status),
        )
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Patched status {:?} for {}", actor.status, actor.name_any());

    Ok(())
}
