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
use kube::{Api, Client, Resource, ResourceExt};
use serde_json::json;

use super::crds::{Actor, ActorSpec, Playbook};
use super::error::{Error, Result};
use crate::resources::crds::ActorState;

pub async fn exists(client: Client, playbook: &Playbook, spec: &ActorSpec) -> Result<bool> {
    let namespace = playbook.spec.namespace.clone();
    let name = spec.name.clone();

    let api: Api<Actor> = Api::namespaced(client, namespace.as_str());
    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: Client, playbook: &Playbook, spec: &ActorSpec) -> Result<Actor> {
    let namespace = playbook.spec.namespace.clone();
    let api: Api<Actor> = Api::namespaced(client.clone(), namespace.as_str());

    let name = spec.name.clone();
    let mut resource = Actor::new(&name, spec.clone());
    resource
        .owner_references_mut()
        .push(playbook.controller_owner_ref(&()).unwrap());
    tracing::debug!("The actor resource:\n {:#?}\n", resource);

    let actor = api
        .create(&PostParams::default(), &resource)
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Created actor: {}", actor.name_any());

    // Patch this actor as initial Pending status
    patch_status(client.clone(), &actor, ActorState::pending()).await?;
    Ok(actor)
}

pub async fn update(client: Client, playbook: &Playbook, spec: &ActorSpec) -> Result<Actor> {
    let namespace = playbook.spec.namespace.clone();
    let api: Api<Actor> = Api::namespaced(client.clone(), namespace.as_str());

    let name = spec.name.clone();
    let mut actor = api.get(&name).await.map_err(Error::KubeError)?;
    tracing::debug!("The Actor {} already exists: {:#?}", &spec.name, actor);

    if &actor.spec != spec {
        let mut resource = Actor::new(&name, spec.clone());
        resource
            .owner_references_mut()
            .push(playbook.controller_owner_ref(&()).unwrap());
        tracing::debug!("The updating actor resource:\n {:#?}\n", resource);

        actor = api
            .patch(
                &name,
                &PatchParams::apply("amp-composer").force(),
                &Patch::Apply(&resource),
            )
            .await
            .map_err(Error::KubeError)?;

        tracing::info!("Updated actor: {}", actor.name_any());
    }

    Ok(actor)
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
