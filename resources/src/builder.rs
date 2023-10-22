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
use amp_common::schema::BuildMethod;
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::PodTemplateSpec;
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};
use tracing::{debug, info};

use crate::containers::{buildpacks, kaniko};
use crate::error::{Error, Result};
use crate::{hash, LAST_APPLIED_HASH_KEY};

pub async fn exists(client: &Client, actor: &Actor) -> Result<bool> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.spec.name();

    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, actor: &Actor) -> Result<Job> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());

    let resource = new(actor)?;
    debug!("The Job resource:\n {:?}\n", resource);

    let job = api.create(&PostParams::default(), &resource).await.map_err(Error::KubeError)?;

    info!("Created Job: {}", job.name_any());
    Ok(job)
}

pub async fn update(client: &Client, actor: &Actor) -> Result<Job> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.spec.name();

    let mut job = api.get(&name).await.map_err(Error::KubeError)?;
    debug!("The Job {} already exists: {:?}", &name, job);

    let expected_hash = hash(&actor.spec)?;
    let found_hash: String = job.annotations().get(LAST_APPLIED_HASH_KEY).map_or("".into(), |v| v.into());

    if found_hash == expected_hash {
        debug!("The Job {} is already up-to-date", &name);
        return Ok(job);
    }

    let resource = new(actor)?;
    debug!("The updating Job resource:\n {:?}\n", resource);

    let params = &PatchParams::apply("amp-controllers").force();
    job = api.patch(&name, params, &Patch::Apply(&resource)).await.map_err(Error::KubeError)?;

    info!("Updated Job: {}", job.name_any());
    Ok(job)
}

/// Create a Job for build images
fn new(actor: &Actor) -> Result<Job> {
    let name = actor.spec.name();

    // Build the metadata for the job
    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let annotations = BTreeMap::from([(LAST_APPLIED_HASH_KEY.into(), hash(&actor.spec)?)]);
    let labels = BTreeMap::from([
        ("app.kubernetes.io/name".into(), name.clone()),
        ("app.kubernetes.io/managed-by".into(), "Amphitheatre".into()),
    ]);
    let metadata = ObjectMeta {
        name: Some(name),
        owner_references: Some(vec![owner_reference]),
        labels: Some(labels.clone()),
        annotations: Some(annotations),
        ..Default::default()
    };

    // Prefer to use Kaniko to build images with Dockerfile,
    // else, build the image with Cloud Native Buildpacks
    let build = actor.spec.character.build.clone().unwrap_or_default();
    let pod = match build.method() {
        BuildMethod::Dockerfile => {
            debug!("Found dockerfile, build it with kaniko");
            kaniko::pod(&actor.spec)
        }
        BuildMethod::Buildpacks => {
            debug!("Build the image with Cloud Native Buildpacks");
            buildpacks::pod(&actor.spec)
        }
    };

    // Build the spec for the job
    let spec = JobSpec {
        backoff_limit: Some(0),
        template: PodTemplateSpec {
            metadata: Some(ObjectMeta { labels: Some(labels), ..Default::default() }),
            spec: Some(pod),
        },
        ..Default::default()
    };

    // Build and return the job resource
    Ok(Job { metadata, spec: Some(spec), ..Default::default() })
}

pub async fn completed(client: &Client, actor: &Actor) -> Result<bool> {
    debug!("Check If the build Job has not completed");

    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.spec.name();

    if let Ok(Some(job)) = api.get_opt(&name).await {
        debug!("Found Job {}", &name);
        Ok(job.status.map_or(false, |s| s.succeeded >= Some(1)))
    } else {
        debug!("Not found Job {}", &name);
        Ok(false)
    }
}
