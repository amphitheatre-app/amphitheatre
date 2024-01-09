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

use super::error::{Error, Result};

use amp_common::resource::{Actor, ActorSpec, ActorState, Playbook};
use k8s_metrics::v1beta1::PodMetrics;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::api::{ListParams, Patch, PatchParams, PostParams};
use kube::{Api, Client, Resource, ResourceExt};
use serde_json::json;
use tracing::{debug, error, info};

pub async fn exists(client: &Client, playbook: &Playbook, name: &str) -> Result<bool> {
    let namespace = playbook.spec.namespace.clone();
    let api: Api<Actor> = Api::namespaced(client.clone(), namespace.as_str());

    Ok(api.get_opt(name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, playbook: &Playbook, spec: &ActorSpec) -> Result<Actor> {
    let namespace = playbook.spec.namespace.clone();
    let api: Api<Actor> = Api::namespaced(client.clone(), namespace.as_str());

    let name = spec.name.clone();
    let mut resource = Actor::new(&name, spec.clone());
    resource.owner_references_mut().push(playbook.controller_owner_ref(&()).unwrap());
    debug!("The Actor resource:\n {:?}\n", resource);

    let actor = api.create(&PostParams::default(), &resource).await.map_err(Error::KubeError)?;

    info!("Created Actor: {}", actor.name_any());

    // Patch this actor as initial Pending status
    patch_status(client, &actor, ActorState::pending()).await?;
    Ok(actor)
}

pub async fn update(client: &Client, playbook: &Playbook, spec: &ActorSpec) -> Result<Actor> {
    let namespace = playbook.spec.namespace.clone();
    let api: Api<Actor> = Api::namespaced(client.clone(), namespace.as_str());

    let name = spec.name.clone();
    let mut actor = api.get(&name).await.map_err(Error::KubeError)?;
    debug!("The Actor {} already exists: {:?}", &spec.name, actor);

    if &actor.spec == spec {
        debug!("The Actor {} is already up-to-date", &spec.name);
        return Ok(actor);
    }

    let mut resource = Actor::new(&name, spec.clone());
    resource.owner_references_mut().push(playbook.controller_owner_ref(&()).unwrap());
    debug!("The updating Actor resource:\n {:?}\n", resource);

    let params = &PatchParams::apply("amp-controllers").force();
    actor = api.patch(&name, params, &Patch::Apply(&resource)).await.map_err(Error::KubeError)?;

    info!("Updated Actor: {}", actor.name_any());

    Ok(actor)
}

pub async fn patch_status(client: &Client, actor: &Actor, condition: Condition) -> Result<()> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;

    let api: Api<Actor> = Api::namespaced(client.clone(), &namespace);

    let status = json!({ "status": { "conditions": vec![condition] }});
    let actor = api
        .patch_status(actor.name_any().as_str(), &PatchParams::default(), &Patch::Merge(&status))
        .await
        .map_err(Error::KubeError)?;

    info!("Patched status {:?} for Actor {}", actor.status, actor.name_any());

    Ok(())
}

pub async fn metrics(client: &Client, namespace: &str, name: &str) -> Result<PodMetrics> {
    let api: Api<PodMetrics> = Api::namespaced(client.clone(), namespace);
    let params = ListParams::default().labels(&format!("app.kubernetes.io/name={}", name)).limit(1);
    let resources = api.list(&params).await;
    debug!("Metrics for Actor {}:\n{:?}", name, resources);

    match resources {
        Ok(resources) => Ok(resources.items.first().ok_or_else(|| Error::MetricsNotAvailable)?.clone()),
        Err(err) => {
            // check if the error is NotFound
            if let kube::Error::Api(error_response) = &err {
                if error_response.code == 404 {
                    error!("No metrics found for Actor {}", name);
                    return Err(Error::MetricsNotAvailable);
                }
            }
            error!("Failed to get metrics for Actor {}: {}", name, err);
            Err(Error::KubeError(err))
        }
    }
}

pub async fn get(client: &Client, namespace: &str, name: &str) -> Result<Actor> {
    let api: Api<Actor> = Api::namespaced(client.clone(), namespace);
    let actor = api.get(name).await.map_err(Error::KubeError)?;

    Ok(actor)
}

pub async fn list(client: &Client, namespace: &str) -> Result<Vec<Actor>> {
    let api: Api<Actor> = Api::namespaced(client.clone(), namespace);
    let actors = api.list(&ListParams::default()).await.map_err(Error::KubeError)?;

    Ok(actors.items)
}
