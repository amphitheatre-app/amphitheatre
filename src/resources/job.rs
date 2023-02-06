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

use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    Container, EmptyDirVolumeSource, PodSpec, PodTemplateSpec, SecurityContext, Volume, VolumeMount,
};
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};

use super::crds::Actor;
use super::error::Result;
use super::{hash, to_env_var, DEFAULT_GITSYNC_IMAGE, DEFAULT_KANIKO_IMAGE, LAST_APPLIED_HASH_KEY};
use crate::resources::error::Error;

pub async fn exists(client: Client, actor: &Actor) -> Result<bool> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client, namespace.as_str());
    let name = actor.build_name();

    Ok(api
        .get_opt(&name)
        .await
        .map_err(Error::KubeError)?
        .is_some())
}

pub async fn create(client: Client, actor: &Actor) -> Result<Job> {
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

pub async fn update(client: Client, actor: &Actor) -> Result<Job> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.build_name();

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
                &PatchParams::apply("amp-builder").force(),
                &Patch::Apply(&resource),
            )
            .await
            .map_err(Error::KubeError)?;

        tracing::info!("Updated Job: {}", job.name_any());
    }

    Ok(job)
}

fn new(actor: &Actor) -> Result<Job> {
    if actor.spec.has_dockerfile() {
        new_kaniko_job(actor)
    } else {
        new_buildpacks_job(actor)
    }
}

fn new_kaniko_job(actor: &Actor) -> Result<Job> {
    let name = actor.build_name();

    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let labels = BTreeMap::from([
        ("app.kubernetes.io/name".into(), name.clone()),
        ("app.kubernetes.io/managed-by".into(), "Amphitheatre".into()),
    ]);
    let annotations = BTreeMap::from([(LAST_APPLIED_HASH_KEY.into(), hash(&actor.spec)?)]);

    let mut args: HashMap<String, String> = HashMap::from([
        ("dockerfile".into(), "Dockerfile".into()),
        ("context".into(), "dir://workspace".into()),
        ("destination".into(), actor.docker_tag()),
        ("verbosity".into(), "debug".into()),
        ("cache".into(), "false".into()),
    ]);

    if let Some(build) = &actor.spec.build {
        if let Some(e) = &build.env {
            args.extend(e.clone())
        }
    }

    let args = args
        .iter()
        .map(|(key, value)| format!("--{}={}", key, value))
        .collect();

    let git_sync_env: HashMap<String, String> = HashMap::from([
        ("GIT_SYNC_REPO".into(), actor.spec.repository.clone()),
        (
            "GIT_SYNC_BRANCH".into(),
            actor.spec.reference.clone().unwrap_or_default(),
        ),
        ("GIT_SYNC_ROOT".into(), "/workspace".into()),
        ("GIT_SYNC_PERMISSIONS".into(), "0777".into()),
        ("GIT_SYNC_ONE_TIME".into(), "true".into()),
        ("GIT_SYNC_SSH".into(), "true".into()),
        ("GIT_KNOWN_HOSTS".into(), "true".into()),
    ]);

    let git_sync_container = Container {
        name: "git-sync".to_string(),
        image: Some(DEFAULT_GITSYNC_IMAGE.to_string()),
        image_pull_policy: Some("Always".into()),
        env: Some(to_env_var(&git_sync_env)),
        volume_mounts: Some(vec![VolumeMount {
            name: "build-context".to_string(),
            mount_path: "/workspace".to_string(),
            ..Default::default()
        }]),
        security_context: Some(SecurityContext {
            run_as_user: Some(0),
            ..Default::default()
        }),
        ..Default::default()
    };

    let kaniko_container = Container {
        name: name.clone(),
        image: Some(DEFAULT_KANIKO_IMAGE.to_string()),
        image_pull_policy: Some("Always".into()),
        args: Some(args),
        volume_mounts: Some(vec![VolumeMount {
            name: "build-context".to_string(),
            mount_path: "/workspace".to_string(),
            ..Default::default()
        }]),
        ..Default::default()
    };

    let template = PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some(labels.clone()),
            ..Default::default()
        }),
        spec: Some(PodSpec {
            restart_policy: Some("Never".into()),
            init_containers: Some(vec![git_sync_container]),
            containers: vec![kaniko_container],
            volumes: Some(vec![Volume {
                name: "build-context".to_string(),
                empty_dir: Some(EmptyDirVolumeSource {
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
            backoff_limit: Some(1),
            completions: Some(1),
            parallelism: Some(1),
            ..Default::default()
        }),
        ..Default::default()
    };
    Ok(resource)
}

fn new_buildpacks_job(_actor: &Actor) -> Result<Job> {
    todo!()
}
