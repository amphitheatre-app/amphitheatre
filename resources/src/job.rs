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

use std::collections::{BTreeMap, HashMap};

use amp_common::schema::{Actor, ActorSpec};
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    Container, KeyToPath, PodSpec, PodTemplateSpec, SecretVolumeSource, Volume, VolumeMount,
};
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};

use super::error::{Error, Result};
use super::{hash, DEFAULT_KANIKO_IMAGE, LAST_APPLIED_HASH_KEY};

pub async fn exists(client: &Client, actor: &Actor) -> Result<bool> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.spec.build_name();

    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, actor: &Actor) -> Result<Job> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());

    let resource = new(actor)?;
    tracing::debug!("The Job resource:\n {:#?}\n", resource);

    let job = api
        .create(&PostParams::default(), &resource)
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Created Job: {}", job.name_any());
    Ok(job)
}

pub async fn update(client: &Client, actor: &Actor) -> Result<Job> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.spec.build_name();

    let mut job = api.get(&name).await.map_err(Error::KubeError)?;
    tracing::debug!("The Job {} already exists: {:#?}", &name, job);

    let expected_hash = hash(&actor.spec)?;
    let found_hash: String = job
        .annotations()
        .get(LAST_APPLIED_HASH_KEY)
        .map_or("".into(), |v| v.into());

    if found_hash != expected_hash {
        let resource = new(actor)?;
        tracing::debug!("The updating Job resource:\n {:#?}\n", resource);

        job = api
            .patch(
                &name,
                &PatchParams::apply("amp-controllers").force(),
                &Patch::Apply(&resource),
            )
            .await
            .map_err(Error::KubeError)?;

        tracing::info!("Updated Job: {}", job.name_any());
    }

    Ok(job)
}

/// Create a Job for build images
fn new(actor: &Actor) -> Result<Job> {
    let name = actor.spec.build_name();
    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let annotations = BTreeMap::from([(LAST_APPLIED_HASH_KEY.into(), hash(&actor.spec)?)]);
    let labels = BTreeMap::from([
        ("app.kubernetes.io/name".into(), name.clone()),
        ("app.kubernetes.io/managed-by".into(), "Amphitheatre".into()),
    ]);

    let container = new_kaniko_container(&actor.spec)?;
    let template = PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some(labels.clone()),
            ..Default::default()
        }),
        spec: Some(PodSpec {
            restart_policy: Some("Never".into()),
            containers: vec![container],
            volumes: Some(vec![Volume {
                name: "kaniko-secret".to_string(),
                secret: Some(SecretVolumeSource {
                    secret_name: Some("amp-registry-credentials".to_string()),
                    items: Some(vec![KeyToPath {
                        key: ".dockerconfigjson".to_string(),
                        path: "config.json".to_string(),
                        ..Default::default()
                    }]),
                    ..Default::default()
                }),
                ..Default::default()
            }]),
            ..Default::default()
        }),
    };

    let resource = Job {
        metadata: ObjectMeta {
            name: Some(name),
            owner_references: Some(vec![owner_reference]),
            labels: Some(labels),
            annotations: Some(annotations),
            ..Default::default()
        },
        spec: Some(JobSpec {
            template,
            backoff_limit: Some(0),
            ..Default::default()
        }),
        ..Default::default()
    };

    Ok(resource)
}

#[inline]
fn context(spec: &ActorSpec) -> String {
    format!(
        "{}#{}",
        spec.source.repo.replace("https", "git"),
        spec.source.rev()
    )
}

fn new_kaniko_container(spec: &ActorSpec) -> Result<Container> {
    let args: HashMap<String, String> = HashMap::from([
        ("context".into(), context(spec)),
        ("destination".into(), spec.docker_tag()),
        ("verbosity".into(), "trace".into()),
        ("cache".into(), "false".into()),
    ]);

    let container = Container {
        name: "build".to_string(),
        image: Some(DEFAULT_KANIKO_IMAGE.to_string()),
        image_pull_policy: Some("Always".into()),
        args: Some(
            args.iter()
                .map(|(key, value)| format!("--{}={}", key, value))
                .collect(),
        ),
        volume_mounts: Some(vec![VolumeMount {
            name: "kaniko-secret".to_string(),
            mount_path: "/kaniko/.docker".to_string(),
            ..Default::default()
        }]),
        env: spec.build_env(),
        ..Default::default()
    };

    Ok(container)
}
