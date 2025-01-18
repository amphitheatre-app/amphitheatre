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
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{PodSpec, PodTemplateSpec};
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};

use crate::error::{Error, Result};
use crate::{hash, LAST_APPLIED_HASH_KEY};

pub async fn exists(client: &Client, actor: &Actor) -> Result<bool> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());
    let name = format!("{}-builder", actor.spec.name);

    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, actor: &Actor, pod: PodSpec) -> Result<Job> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());

    let resource = new(actor, pod)?;
    let job = api.create(&PostParams::default(), &resource).await.map_err(Error::KubeError)?;
    tracing::info!("Created Job: {}", job.name_any());

    Ok(job)
}

pub async fn update(client: &Client, actor: &Actor, pod: PodSpec) -> Result<Job> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());
    let name = format!("{}-builder", actor.spec.name);

    let mut job = api.get(&name).await.map_err(Error::KubeError)?;
    tracing::debug!("The Job {} already exists", &name);

    let expected_hash = hash(&actor.spec)?;
    let found_hash: String = job.annotations().get(LAST_APPLIED_HASH_KEY).map_or("".into(), |v| v.into());

    if found_hash != expected_hash {
        let resource = new(actor, pod)?;
        tracing::debug!("The updating Job resource:\n {:?}\n", resource);

        job = api
            .patch(&name, &PatchParams::apply("amp-controllers").force(), &Patch::Apply(&resource))
            .await
            .map_err(Error::KubeError)?;

        tracing::info!("Updated Job: {}", job.name_any());
    }

    Ok(job)
}

/// Create a Job for build images
fn new(actor: &Actor, pod: PodSpec) -> Result<Job> {
    let name = format!("{}-builder", actor.spec.name);
    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let annotations = BTreeMap::from([(LAST_APPLIED_HASH_KEY.into(), hash(&actor.spec)?)]);
    let labels = BTreeMap::from([
        ("amphitheatre.app/character".into(), actor.spec.name.clone()),
        ("app.kubernetes.io/managed-by".into(), "Amphitheatre".into()),
    ]);

    Ok(Job {
        metadata: ObjectMeta {
            name: Some(name),
            owner_references: Some(vec![owner_reference]),
            labels: Some(labels.clone()),
            annotations: Some(annotations),
            ..Default::default()
        },
        spec: Some(JobSpec {
            backoff_limit: Some(0),
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta { labels: Some(labels), ..Default::default() }),
                spec: Some(pod),
            },
            ..Default::default()
        }),
        ..Default::default()
    })
}

pub async fn completed(client: &Client, actor: &Actor) -> Result<bool> {
    tracing::debug!("Check If the build Job has not completed");

    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Job> = Api::namespaced(client.clone(), namespace.as_str());
    let name = format!("{}-builder", actor.spec.name);

    if let Ok(Some(job)) = api.get_opt(&name).await {
        tracing::debug!("Found Job {}", &name);
        Ok(job.status.is_some_and(|s| s.succeeded >= Some(1)))
    } else {
        tracing::debug!("Not found Job {}", &name);
        Ok(false)
    }
}
