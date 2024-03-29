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
use k8s_openapi::api::core::v1::{PersistentVolumeClaimVolumeSource, Pod, PodSpec, Volume};
use kube::api::{Patch, PatchParams, PostParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};
use tracing::{debug, info};

use crate::containers::syncer;
use crate::error::{Error, Result};
use crate::{hash, LAST_APPLIED_HASH_KEY};

use super::BuildExt;

pub async fn exists(client: &Client, actor: &Actor) -> Result<bool> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Pod> = Api::namespaced(client.clone(), namespace.as_str());
    let name = format!("{}-syncer", actor.spec.name);

    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, actor: &Actor) -> Result<Pod> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Pod> = Api::namespaced(client.clone(), namespace.as_str());

    let resource = new(actor)?;
    let pod = api.create(&PostParams::default(), &resource).await.map_err(Error::KubeError)?;
    info!("Created Pod: {}", pod.name_any());

    Ok(pod)
}

pub async fn update(client: &Client, actor: &Actor) -> Result<Pod> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Pod> = Api::namespaced(client.clone(), namespace.as_str());
    let name = format!("{}-syncer", actor.spec.name);

    let mut pod = api.get(&name).await.map_err(Error::KubeError)?;
    debug!("The Pod {} already exists", &name);

    let expected_hash = hash(&actor.spec)?;
    let found_hash: String = pod.annotations().get(LAST_APPLIED_HASH_KEY).map_or("".into(), |v| v.into());

    if found_hash != expected_hash {
        let resource = new(actor)?;
        debug!("The updating syncer pod resource:\n {:?}\n", resource);

        pod = api
            .patch(&name, &PatchParams::apply("amp-controllers").force(), &Patch::Apply(&resource))
            .await
            .map_err(Error::KubeError)?;

        info!("Updated Pod: {}", pod.name_any());
    }

    Ok(pod)
}

/// Create a Syncer Pod for build images
fn new(actor: &Actor) -> Result<Pod> {
    let name = format!("{}-syncer", actor.spec.name);
    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let annotations = BTreeMap::from([(LAST_APPLIED_HASH_KEY.into(), hash(&actor.spec)?)]);
    let labels = BTreeMap::from([
        ("amphitheatre.app/character".into(), actor.spec.name.clone()),
        ("app.kubernetes.io/managed-by".into(), "Amphitheatre".into()),
    ]);

    Ok(Pod {
        metadata: ObjectMeta {
            name: Some(name),
            owner_references: Some(vec![owner_reference]),
            labels: Some(labels.clone()),
            annotations: Some(annotations),
            ..Default::default()
        },
        spec: Some(PodSpec {
            containers: vec![syncer::container(actor, &None)?],
            restart_policy: Some("Never".into()),
            volumes: Some(vec![Volume {
                name: "workspace".to_string(),
                persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                    claim_name: actor.spec.character.pvc_name(),
                    read_only: Some(false),
                }),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    })
}

pub async fn ready(client: &Client, actor: &Actor) -> Result<bool> {
    debug!("Check If the syncer Pod is ready");

    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<Pod> = Api::namespaced(client.clone(), namespace.as_str());
    let name = format!("{}-syncer", actor.spec.name);

    let result = api.get_opt(&name).await.map_err(Error::KubeError)?;
    if result.is_none() {
        debug!("Not found Syncer {}", &name);
        return Ok(false);
    }

    let pod = result.unwrap();
    let status = pod.status.as_ref().ok_or_else(|| Error::MissingObjectKey(".status"))?;

    // Check if the Pod phase is Succeeded
    if let Some(phase) = &status.phase {
        return Ok(phase == "Succeeded");
    }

    if let Some(conditions) = &status.conditions {
        return Ok(conditions.iter().any(|c| c.type_ == "Running" && c.status == "True"));
    }

    Ok(false)
}
