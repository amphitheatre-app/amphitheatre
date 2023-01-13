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

use super::crds::{ActorSpec, Playbook};
use super::deployment;
use super::error::{Error, Result};

pub async fn build(client: Client, playbook: &Playbook, actor: &ActorSpec) -> Result<()> {
    let namespace = playbook
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
            "name": format!("{}-{}", actor.name, actor.commit),
        },
        "spec": {
            "tag": format!("harbor.amp-system.svc.cluster.local/library/{}:{}", actor.image, actor.commit),
            "serviceAccountName": "default",
            "builder": {
                "name": "amp-default-cluster-builder",
                "kind": "ClusterBuilder",
            },
            "source": {
                "git": {
                    "url": actor.repository,
                    "revision": actor.commit,
                },
                "subPath": actor.path,
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

pub async fn add(client: Client, playbook: &Playbook, actor: ActorSpec) -> Result<()> {
    let api: Api<Playbook> = Api::all(client);

    let actor_name = actor.name.clone();
    let mut actors = playbook.spec.actors.clone();
    actors.push(actor);

    let patch = json!({"spec": { "actors": actors }});
    let playbook = api
        .patch(
            playbook.name_any().as_str(),
            &PatchParams::apply("amp-composer"),
            &Patch::Merge(&patch),
        )
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Added actor {:?} for {}", actor_name, playbook.name_any());

    Ok(())
}

pub async fn deploy(client: Client, playbook: &Playbook, actor: &ActorSpec) -> Result<()> {
    // Create Deployment resource for this actor
    deployment::create(client, playbook, actor).await?;

    // TODO: Create Service resource if needed.

    Ok(())
}
