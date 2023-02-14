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

use super::crds::{Actor, ActorSpec};
use super::error::Result;
use super::{
    hash, to_env_var, DEFAULT_BP_IMAGE, DEFAULT_GITSYNC_IMAGE, DEFAULT_KANIKO_IMAGE,
    LAST_APPLIED_HASH_KEY,
};
use crate::resources::error::Error;

pub async fn exists(client: Client, actor: &Actor) -> Result<bool> {
    let namespace = actor
        .namespace()
        .ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client, namespace.as_str());
    let name = actor.spec.build_name();

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
                &PatchParams::apply("amp-builder").force(),
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

    // Create a init container for clones the git repo to workspace
    let git_sync_container = new_git_sync_container(&actor.spec)?;

    // Prefer to use Kaniko to build images with Dockerfile,
    // else, build the image with Cloud Native Buildpacks
    let container = if actor.spec.has_dockerfile() {
        new_kaniko_container(&actor.spec)?
    } else {
        new_buildpacks_container(&actor.spec)?
    };

    let template = PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some(labels.clone()),
            ..Default::default()
        }),
        spec: Some(PodSpec {
            restart_policy: Some("Never".into()),
            init_containers: Some(vec![git_sync_container]),
            containers: vec![container],
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

/// A sidecar app which clones a git repo and keeps it in sync with the upstream.
///
/// Many options can be specified as an environment variable.
/// See https://github.com/kubernetes/git-sync#manual
fn new_git_sync_container(spec: &ActorSpec) -> Result<Container> {
    let env: HashMap<String, String> = HashMap::from([
        // The git repository to sync.  This flag is required.
        ("GIT_SYNC_REPO".into(), spec.repository.clone()),
        // The git branch to check out. If not specified, this defaults to
        // the default branch of --repo.
        (
            "GIT_SYNC_BRANCH".into(),
            spec.reference.clone().unwrap_or_default(),
        ),
        // The git revision (tag or hash) to check out.  If not specified,
        // this defaults to "HEAD".
        ("GIT_SYNC_REV".into(), spec.commit.clone()),
        // Create a shallow clone with history truncated to the specified
        // number of commits.  If not specified, this defaults to cloning the
        // full history of the repo.
        ("GIT_SYNC_DEPTH".into(), "1".into()),
        // Exit after one sync.
        ("GIT_SYNC_ONE_TIME".into(), "true".into()),
        // The root directory for git-sync operations, under which --link will
        // be created.  This must be a path that either a) does not exist (it
        // will be created); b) is an empty directory; or c) is a directory
        // which can be emptied by removing all of the contents.  This flag is
        // required.
        ("GIT_SYNC_ROOT".into(), "/workspace".into()),
        // Change permissions on the checked-out files to the specified mode.
        ("GIT_SYNC_PERMISSIONS".into(), "0777".into()),
        // Use SSH for git authentication and operations.
        ("GIT_SYNC_SSH".into(), "false".into()),
        // Enable SSH known_hosts verification when using --ssh.  If not
        // specified, this defaults to true.
        ("GIT_KNOWN_HOSTS".into(), "true".into()),
    ]);

    let container = Container {
        name: "git-sync".to_string(),
        image: Some(DEFAULT_GITSYNC_IMAGE.to_string()),
        image_pull_policy: Some("Always".into()),
        env: Some(to_env_var(&env)),
        volume_mounts: Some(vec![workspace()]),
        security_context: Some(SecurityContext {
            run_as_user: Some(0),
            ..Default::default()
        }),
        ..Default::default()
    };

    Ok(container)
}

fn new_kaniko_container(spec: &ActorSpec) -> Result<Container> {
    let args: HashMap<String, String> = HashMap::from([
        ("dockerfile".into(), spec.dockerfile()),
        ("context".into(), "dir://workspace".into()),
        ("destination".into(), spec.docker_tag()),
        ("verbosity".into(), "debug".into()),
        ("cache".into(), "false".into()),
    ]);

    let container = Container {
        name: spec.build_name(),
        image: Some(DEFAULT_KANIKO_IMAGE.to_string()),
        image_pull_policy: Some("Always".into()),
        args: Some(
            args.iter()
                .map(|(key, value)| format!("--{}={}", key, value))
                .collect(),
        ),
        env: spec.build_env(),
        volume_mounts: Some(vec![workspace()]),
        ..Default::default()
    };

    Ok(container)
}

fn new_buildpacks_container(spec: &ActorSpec) -> Result<Container> {
    let args: Vec<String> = vec![
        // Path to application directory
        format!("-app=/workspace/{}", spec.context()),
        // Log Level
        "-log-level=info".to_string(),
        // Primary GID of the build image User
        "-gid=1000".to_string(),
        // UID of the build image User
        "-uid=1000".to_string(),
    ];

    let container = Container {
        name: spec.build_name(),
        image: Some(DEFAULT_BP_IMAGE.to_string()),
        image_pull_policy: Some("Always".into()),
        command: Some(vec![
            // Running creator SHALL be equivalent to running detector, analyzer, restorer, builder
            // and exporter in order with identical inputs where they are accepted
            "/cnb/lifecycle/creator".to_string(),
            // Tag reference to which the app image will be written
            spec.docker_tag(),
        ]),
        args: Some(args),
        env: spec.build_env(),
        volume_mounts: Some(vec![workspace()]),
        ..Default::default()
    };

    Ok(container)
}

fn workspace() -> VolumeMount {
    VolumeMount {
        name: "build-context".to_string(),
        mount_path: "/workspace".to_string(),
        ..Default::default()
    }
}
