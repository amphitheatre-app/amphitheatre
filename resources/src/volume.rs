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
use k8s_openapi::api::core::v1::{PersistentVolumeClaim, PersistentVolumeClaimSpec, ResourceRequirements};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use kube::api::PostParams;
use kube::core::ObjectMeta;
use kube::{Api, Client, Resource, ResourceExt};
use tracing::{debug, info};

use crate::error::{Error, Result};
use crate::kpack::BuildExt;

pub async fn exists(client: &Client, actor: &Actor) -> Result<bool> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), namespace.as_str());
    let name = actor.spec.character.pvc_name();

    Ok(api.get_opt(&name).await.map_err(Error::KubeError)?.is_some())
}

pub async fn create(client: &Client, actor: &Actor) -> Result<PersistentVolumeClaim> {
    let namespace = actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;
    let api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), namespace.as_str());

    let resource = new(actor)?;
    debug!("The creating PersistentVolumeClaim resource:\n {:?}\n", resource);

    let pvc = api.create(&PostParams::default(), &resource).await.map_err(Error::KubeError)?;
    info!("Created PersistentVolumeClaim: {}", pvc.name_any());

    Ok(pvc)
}

fn new(actor: &Actor) -> Result<PersistentVolumeClaim> {
    let name = actor.spec.character.pvc_name();
    let owner_reference = actor.controller_owner_ref(&()).unwrap();
    let labels = BTreeMap::from([
        ("amphitheatre.app/character".into(), actor.spec.name.clone()),
        ("app.kubernetes.io/managed-by".into(), "Amphitheatre".into()),
    ]);

    Ok(PersistentVolumeClaim {
        metadata: ObjectMeta {
            name: Some(name),
            owner_references: Some(vec![owner_reference]),
            labels: Some(labels.clone()),
            ..Default::default()
        },
        spec: Some(PersistentVolumeClaimSpec {
            access_modes: Some(vec!["ReadWriteOnce".into()]),
            resources: Some(ResourceRequirements {
                requests: Some(BTreeMap::from([("storage".into(), Quantity("1Gi".into()))])),
                ..Default::default()
            }),
            // @TODO: Make storage class name configurable
            // storage_class_name: Some("local-path".into()),
            volume_mode: Some("Filesystem".into()),
            ..Default::default()
        }),
        ..Default::default()
    })
}
