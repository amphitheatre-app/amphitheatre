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

use std::collections::BTreeMap;

use k8s_openapi::api::core::v1::{Service, ServiceSpec};
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};

use super::crds::Actor;
use super::error::Result;
use super::{hash, LAST_APPLIED_HASH_KEY};
use crate::resources::error::Error;

pub async fn exists(client: Client, actor: &Actor) -> Result<bool> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Service> = Api::namespaced(client, namespace.as_str());
    let name = actor.name_any();

    Ok(api
        .get_opt(&name)
        .await
        .map_err(Error::KubeError)?
        .is_some())
}

pub async fn create(client: Client, actor: &Actor) -> Result<Service> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Service> = Api::namespaced(client.clone(), namespace.as_str());

    let resource = new(actor)?;
    tracing::debug!("The service resource:\n {:#?}\n", resource);

    let service = api
        .create(&PostParams::default(), &resource)
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Created service: {}", service.name_any());
    Ok(service)
}

pub async fn update(client: Client, actor: &Actor) -> Result<Service> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Service> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.name_any();

    let mut service = api.get(&name).await.map_err(Error::KubeError)?;
    tracing::debug!("The Service {} already exists: {:#?}", &name, service);

    let expected_hash = hash(&actor.spec)?;
    let found_hash: String = service
        .annotations()
        .get(LAST_APPLIED_HASH_KEY)
        .map_or("".into(), |v| v.into());

    if found_hash != expected_hash {
        let resource = new(actor)?;
        tracing::debug!("The updating Service resource:\n {:#?}\n", resource);

        service = api
            .patch(
                &name,
                &PatchParams::apply("amp-composer").force(),
                &Patch::Apply(&resource),
            )
            .await
            .map_err(Error::KubeError)?;

        tracing::info!("Updated Service: {}", service.name_any());
    }

    Ok(service)
}

fn new(actor: &Actor) -> Result<Service> {
    let name = actor.name_any();

    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let labels = BTreeMap::from([
        ("app.kubernetes.io/name".into(), name.clone()),
        ("app.kubernetes.io/managed-by".into(), "Amphitheatre".into()),
    ]);
    let annotations = BTreeMap::from([(LAST_APPLIED_HASH_KEY.into(), hash(&actor.spec)?)]);

    let resource = Service {
        metadata: ObjectMeta {
            name: Some(name),
            owner_references: Some(vec![owner_reference]),
            labels: Some(labels.clone()),
            annotations: Some(annotations),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(labels),
            ports: actor.spec.service_ports(),
            ..Default::default()
        }),
        ..Default::default()
    };
    Ok(resource)
}
