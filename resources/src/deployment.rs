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
use k8s_openapi::api::core::v1::{PodSpec, PodTemplateSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};
use tracing::{debug, info};

use super::error::{Error, Result};
use super::{hash, LAST_APPLIED_HASH_KEY};

pub async fn exists(client: &Client, namespace: &str, name: &str) -> Result<bool> {
    let api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    Ok(api.get_opt(name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, namespace: &str, resource: Deployment) -> Result<Deployment> {
    let api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    let deployment = api.create(&PostParams::default(), &resource).await.map_err(Error::KubeError)?;
    info!("Created Deployment: {}", deployment.name_any());

    Ok(deployment)
}

pub async fn update(
    client: &Client,
    namespace: &str,
    name: &str,
    resource: Deployment,
    expected_hash: String,
) -> Result<Deployment> {
    let api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    let mut deployment = api.get(name).await.map_err(Error::KubeError)?;
    debug!("The Deployment {} already exists: {:?}", name, deployment);

    let found_hash: String = deployment.annotations().get(LAST_APPLIED_HASH_KEY).map_or("".into(), |v| v.into());

    if found_hash == expected_hash {
        debug!("The Deployment {} is already up-to-date", name);
        return Ok(deployment);
    }

    debug!("The updating Deployment resource:\n {:?}\n", resource);

    let params = &PatchParams::apply("amp-controllers").force();
    deployment = api.patch(name, params, &Patch::Apply(&resource)).await.map_err(Error::KubeError)?;

    info!("Updated Deployment: {}", deployment.name_any());
    Ok(deployment)
}

pub fn new(actor: &Actor, pod: PodSpec) -> Result<Deployment> {
    let name = actor.name_any();

    // Build the metadata for the deployment
    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let labels = BTreeMap::from([
        ("app.kubernetes.io/name".into(), name.clone()),
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

    // Build the spec for the deployment
    let spec = DeploymentSpec {
        selector: LabelSelector { match_labels: Some(labels.clone()), ..Default::default() },
        template: PodTemplateSpec {
            metadata: Some(ObjectMeta { labels: Some(labels.clone()), ..Default::default() }),
            spec: Some(pod),
        },
        ..Default::default()
    };

    // Build and return the deployment resource
    Ok(Deployment { metadata, spec: Some(spec), ..Default::default() })
}
