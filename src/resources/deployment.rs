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

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, PodSpec, PodTemplateSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};
use serde::Serialize;
use serde_json::to_string;
use sha2::{Digest, Sha256};

use super::crds::Actor;
use super::error::Result;
use crate::resources::error::Error;

pub async fn exists(client: Client, actor: &Actor) -> Result<bool> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Deployment> = Api::namespaced(client, namespace.as_str());
    let name = actor.deployment_name();

    Ok(api
        .get_opt(&name)
        .await
        .map_err(Error::KubeError)?
        .is_some())
}

pub async fn create(client: Client, actor: &Actor) -> Result<Deployment> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Deployment> = Api::namespaced(client.clone(), namespace.as_str());

    let resource = new(actor)?;
    tracing::debug!("The deployment resource:\n {:#?}\n", resource);

    let deployment = api
        .create(&PostParams::default(), &resource)
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Created deployment: {}", deployment.name_any());
    Ok(deployment)
}

pub async fn update(client: Client, actor: &Actor) -> Result<Deployment> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Deployment> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.deployment_name();

    let mut deployment = api.get(&name).await.map_err(Error::KubeError)?;
    tracing::debug!("The Deployment {} already exists: {:#?}", &name, deployment);

    let expected_hash = hash(&actor.spec)?;
    let found_hash: String = deployment
        .annotations()
        .get(LAST_APPLIED_HASH_KEY)
        .map_or("".into(), |v| v.into());

    if found_hash != expected_hash {
        let resource = new(actor)?;
        tracing::debug!("The updating deployment resource:\n {:#?}\n", resource);

        deployment = api
            .patch(
                &name,
                &PatchParams::apply("amp-composer").force(),
                &Patch::Apply(&resource),
            )
            .await
            .map_err(Error::KubeError)?;

        tracing::info!("Updated deployment: {}", deployment.name_any());
    }

    Ok(deployment)
}

fn hash<T>(resource: &T) -> Result<String>
where
    T: Serialize,
{
    let data = to_string(resource).map_err(Error::SerializationError)?;
    let hash = Sha256::digest(data);

    Ok(format!("{:x}", hash))
}

const LAST_APPLIED_HASH_KEY: &str = "actors.amphitheatre.io/last-applied-hash";

fn new(actor: &Actor) -> Result<Deployment> {
    let name = actor.deployment_name();

    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let labels = BTreeMap::from([
        ("app.kubernetes.io/name".into(), name.clone()),
        ("app.kubernetes.io/managed-by".into(), "Amphitheatre".into()),
    ]);
    let annotations = BTreeMap::from([(LAST_APPLIED_HASH_KEY.into(), hash(&actor.spec)?)]);

    let container = Container {
        name: name.clone(),
        image: Some(actor.docker_tag()),
        image_pull_policy: Some("Always".into()),
        ..Default::default()
    };

    let template = PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some(labels.clone()),
            ..Default::default()
        }),
        spec: Some(PodSpec {
            containers: vec![container],
            ..Default::default()
        }),
    };

    let resource = Deployment {
        metadata: ObjectMeta {
            name: Some(name),
            owner_references: Some(vec![owner_reference]),
            labels: Some(labels.clone()),
            annotations: Some(annotations),
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            selector: LabelSelector {
                match_labels: Some(labels),
                ..Default::default()
            },
            template,
            ..Default::default()
        }),
        ..Default::default()
    };
    Ok(resource)
}
