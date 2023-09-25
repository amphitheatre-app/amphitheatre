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

use amp_common::resource::Actor;
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, PodSpec, PodTemplateSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};

use super::error::{Error, Result};
use super::{hash, LAST_APPLIED_HASH_KEY};

pub async fn exists(client: &Client, actor: &Actor) -> Result<bool> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Deployment> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.name_any();

    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, actor: &Actor) -> Result<Deployment> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Deployment> = Api::namespaced(client.clone(), namespace.as_str());

    let resource = new(actor)?;
    tracing::debug!("The Deployment resource:\n {:?}\n", resource);

    let deployment = api
        .create(&PostParams::default(), &resource)
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Created Deployment: {}", deployment.name_any());
    Ok(deployment)
}

pub async fn update(client: &Client, actor: &Actor) -> Result<Deployment> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Deployment> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.name_any();

    let mut deployment = api.get(&name).await.map_err(Error::KubeError)?;
    tracing::debug!("The Deployment {} already exists: {:?}", &name, deployment);

    let expected_hash = hash(&actor.spec)?;
    let found_hash: String = deployment
        .annotations()
        .get(LAST_APPLIED_HASH_KEY)
        .map_or("".into(), |v| v.into());

    if found_hash != expected_hash {
        let resource = new(actor)?;
        tracing::debug!("The updating Deployment resource:\n {:?}\n", resource);

        deployment = api
            .patch(
                &name,
                &PatchParams::apply("amp-controllers").force(),
                &Patch::Apply(&resource),
            )
            .await
            .map_err(Error::KubeError)?;

        tracing::info!("Updated Deployment: {}", deployment.name_any());
    }

    Ok(deployment)
}

fn new(actor: &Actor) -> Result<Deployment> {
    let name = actor.name_any();

    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let labels = BTreeMap::from([
        ("app.kubernetes.io/name".into(), name.clone()),
        ("app.kubernetes.io/managed-by".into(), "Amphitheatre".into()),
    ]);
    let annotations = BTreeMap::from([(LAST_APPLIED_HASH_KEY.into(), hash(&actor.spec)?)]);

    // extract the env and ports from the deploy spec
    let mut env = Some(vec![]);
    let mut ports = Some(vec![]);
    if let Some(deploy) = &actor.spec.character.deploy {
        env = deploy.env().clone();
        ports = deploy.container_ports();
    }

    let container = Container {
        name: name.clone(),
        image: Some(actor.spec.image.clone()),
        image_pull_policy: Some("Always".into()),
        env,
        ports,
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
