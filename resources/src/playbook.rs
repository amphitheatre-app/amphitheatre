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

use std::time::Duration;

use amp_common::resource::{CharacterSpec, Playbook, PlaybookState};

use k8s_openapi::apiextensions_apiserver as server;
use server::pkg::apis::apiextensions::v1::CustomResourceDefinition;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::api::{DeleteParams, ListParams, Patch, PatchParams, PostParams};
use kube::core::ObjectList;
use kube::{Api, Client, CustomResourceExt, ResourceExt};

use serde_json::{json, to_string_pretty};
use tokio::time::sleep;
use tracing::{debug, info};

use super::error::{Error, Result};

pub async fn install(client: &Client) -> Result<()> {
    let api: Api<CustomResourceDefinition> = Api::all(client.clone());
    let crd = Playbook::crd();
    debug!("Creating the Playbook CustomResourceDefinition: {}", to_string_pretty(&crd).unwrap());

    let params = PostParams::default();
    match api.create(&params, &crd).await {
        Ok(o) => {
            debug!("Created {} ({:?})", o.name_any(), o.status.unwrap());
            debug!("Created CRD: {:?}", o.spec);
        }
        Err(kube::Error::Api(err)) => assert_eq!(err.code, 409), /* if you skipped delete, for instance */
        Err(err) => return Err(Error::KubeError(err)),
    }

    // Wait for the api to catch up
    sleep(Duration::from_secs(1)).await;

    Ok(())
}

pub async fn uninstall(client: &Client) -> Result<()> {
    let api: Api<CustomResourceDefinition> = Api::all(client.clone());
    let params = DeleteParams::default();

    // Ignore delete error if not exists
    let name = "playbooks.amphitheatre.app";
    let _ = api
        .delete(name, &params)
        .await
        .map_err(Error::KubeError)?
        .map_left(|o| debug!("Deleting CRD: {:?}", o.status))
        .map_right(|s| debug!("Deleted CRD: {:?}", s));

    // Wait for the delete to take place (map-left case or delete from previous run)
    sleep(Duration::from_secs(2)).await;

    Ok(())
}

pub async fn create(client: &Client, playbook: &Playbook) -> Result<Playbook> {
    let api: Api<Playbook> = Api::all(client.clone());

    let playbook = api.create(&PostParams::default(), playbook).await.map_err(Error::KubeError)?;
    info!("Created playbook: {}", playbook.name_any());

    // Patch this playbook as initial Pending status
    patch_status(client, &playbook, PlaybookState::pending()).await?;
    Ok(playbook)
}

pub async fn add(client: &Client, playbook: &Playbook, character: CharacterSpec) -> Result<()> {
    let api: Api<Playbook> = Api::all(client.clone());
    let character_name = character.meta.name.clone();

    let mut characters: Vec<CharacterSpec> = vec![];
    if let Some(items) = &playbook.spec.characters {
        characters = items.clone();
    }
    characters.push(character);

    let params = &PatchParams::apply("amp-controllers");
    let patch = json!({"spec": { "characters": characters }});
    let playbook = api.patch(&playbook.name_any(), params, &Patch::Merge(&patch)).await.map_err(Error::KubeError)?;

    info!("Added character {:?} to {}", character_name, playbook.name_any());

    Ok(())
}

pub async fn patch_status(client: &Client, playbook: &Playbook, condition: Condition) -> Result<()> {
    let api: Api<Playbook> = Api::all(client.clone());

    let status = json!({ "status": { "conditions": vec![condition.clone()] }});
    let playbook = api
        .patch_status(playbook.name_any().as_str(), &PatchParams::default(), &Patch::Merge(&status))
        .await
        .map_err(Error::KubeError)?;
    info!("Patched status {:?} with reason {:?} for Actor {}", condition.type_, condition.reason, playbook.name_any());

    Ok(())
}

/// List all playbooks
pub async fn list(client: &Client) -> Result<ObjectList<Playbook>> {
    let api: Api<Playbook> = Api::all(client.clone());
    let resources = api.list(&ListParams::default()).await.map_err(Error::KubeError)?;

    Ok(resources)
}

/// Get a playbook by name
pub async fn get(client: &Client, name: &str) -> Result<Playbook> {
    let api: Api<Playbook> = Api::all(client.clone());
    let resources = api.get(name).await.map_err(Error::KubeError)?;

    Ok(resources)
}

/// Delete a playbook by name
pub async fn delete(client: &Client, name: &str) -> Result<()> {
    let api: Api<Playbook> = Api::all(client.clone());
    let params = DeleteParams::default();

    // Ignore delete error if not exists
    let _ = api
        .delete(name, &params)
        .await
        .map_err(Error::KubeError)?
        .map_left(|o| debug!("Deleting Playbook: {:?}", o.status))
        .map_right(|s| debug!("Deleted Playbook: {:?}", s));

    // Wait for the delete to take place (map-left case or delete from previous run)
    sleep(Duration::from_secs(2)).await;

    Ok(())
}
