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

use std::collections::BTreeMap;

use amp_common::resource::Actor;
use k8s_openapi::api::core::v1::{Service, ServiceSpec};
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};
use tracing::debug;

use super::error::{Error, Result};
use super::{hash, LAST_APPLIED_HASH_KEY};

pub async fn exists(client: &Client, actor: &Actor) -> Result<bool> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Service> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.name_any();

    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, actor: &Actor) -> Result<Service> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Service> = Api::namespaced(client.clone(), namespace.as_str());

    let resource = new(actor)?;
    tracing::debug!("The service resource:\n {:?}\n", resource);

    let service = api.create(&PostParams::default(), &resource).await.map_err(Error::KubeError)?;

    tracing::info!("Created service: {}", service.name_any());
    Ok(service)
}

pub async fn update(client: &Client, actor: &Actor) -> Result<Service> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Service> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.name_any();

    let mut service = api.get(&name).await.map_err(Error::KubeError)?;
    tracing::debug!("The Service {} already exists: {:?}", &name, service);

    let expected_hash = hash(&actor.spec)?;
    let found_hash: String = service.annotations().get(LAST_APPLIED_HASH_KEY).map_or("".into(), |v| v.into());

    if found_hash == expected_hash {
        debug!("The Service {} is already up-to-date", &name);
        return Ok(service);
    }

    let resource = new(actor)?;
    tracing::debug!("The updating Service resource:\n {:?}\n", resource);

    let params = &PatchParams::apply("amp-controllers").force();
    service = api.patch(&name, params, &Patch::Apply(&resource)).await.map_err(Error::KubeError)?;

    tracing::info!("Updated Service: {}", service.name_any());
    Ok(service)
}

fn new(actor: &Actor) -> Result<Service> {
    let name = actor.name_any();

    // Build the metadata for the service
    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let labels = BTreeMap::from([
        ("amphitheatre.app/character".into(), name.clone()),
        ("app.kubernetes.io/managed-by".into(), "Amphitheatre".into()),
    ]);
    let annotations = BTreeMap::from([(LAST_APPLIED_HASH_KEY.into(), hash(&actor.spec)?)]);
    let metadata = ObjectMeta {
        name: Some(name),
        owner_references: Some(vec![owner_reference]),
        labels: Some(labels.clone()),
        annotations: Some(annotations),
        ..Default::default()
    };

    // Extract ports from deploy spec.
    let mut service_ports = Some(vec![]);
    if let Some(deploy) = &actor.spec.character.deploy {
        service_ports = deploy.service_ports();
    }

    // Build and return the service resource.
    Ok(Service {
        metadata,
        spec: Some(ServiceSpec { selector: Some(labels), ports: service_ports, ..Default::default() }),
        ..Default::default()
    })
}
