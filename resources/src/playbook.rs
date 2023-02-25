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

use std::time::Duration;

use amp_crds::actor::ActorSpec;
use amp_crds::playbook::{Playbook, PlaybookState};
use k8s_openapi::apiextensions_apiserver as server;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::api::{DeleteParams, Patch, PatchParams, PostParams};
use kube::{Api, Client, CustomResourceExt, ResourceExt};
use serde_json::{json, to_string_pretty};
use server::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use tokio::time::sleep;

use super::error::{Error, Result};

pub async fn install(client: &Client) -> Result<()> {
    let api: Api<CustomResourceDefinition> = Api::all(client.clone());
    let crd = Playbook::crd();
    tracing::debug!(
        "Creating the Playbook CustomResourceDefinition: {}",
        to_string_pretty(&crd).unwrap()
    );

    let params = PostParams::default();
    match api.create(&params, &crd).await {
        Ok(o) => {
            tracing::debug!("Created {} ({:?})", o.name_any(), o.status.unwrap());
            tracing::debug!("Created CRD: {:?}", o.spec);
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
        .map_left(|o| tracing::debug!("Deleting CRD: {:?}", o.status))
        .map_right(|s| tracing::debug!("Deleted CRD: {:?}", s));

    // Wait for the delete to take place (map-left case or delete from previous run)
    sleep(Duration::from_secs(2)).await;

    Ok(())
}

pub async fn create(client: &Client, playbook: &Playbook) -> Result<Playbook> {
    let api: Api<Playbook> = Api::all(client.clone());

    tracing::debug!("The playbook resource:\n {:#?}\n", playbook);

    let playbook = api
        .create(&PostParams::default(), playbook)
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Created playbook: {}", playbook.name_any());

    // Patch this playbook as initial Pending status
    patch_status(client, &playbook, PlaybookState::pending()).await?;
    Ok(playbook)
}

pub async fn add(client: &Client, playbook: &Playbook, actor: ActorSpec) -> Result<()> {
    let api: Api<Playbook> = Api::all(client.clone());

    let actor_name = actor.name.clone();
    let mut actors = playbook.spec.actors.clone();
    actors.push(actor);

    let patch = json!({"spec": { "actors": actors }});
    let playbook = api
        .patch(
            playbook.name_any().as_str(),
            &PatchParams::apply("amp-composer"),
            &Patch::Merge(&patch),
        )
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Added actor {:?} for {}", actor_name, playbook.name_any());

    Ok(())
}

pub async fn patch_status(client: &Client, playbook: &Playbook, condition: Condition) -> Result<()> {
    let api: Api<Playbook> = Api::all(client.clone());

    let status = json!({ "status": { "conditions": vec![condition] }});
    let playbook = api
        .patch_status(
            playbook.name_any().as_str(),
            &PatchParams::default(),
            &Patch::Merge(&status),
        )
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Patched status {:?} for {}", playbook.status, playbook.name_any());

    Ok(())
}

pub async fn replace_status(_client: &Client, _playbook: &Playbook, _condition: Condition) -> Result<()> {
    todo!()
}

pub async fn replace_status_with(
    _client: &Client,
    _playbook: &Playbook,
    _before: Condition,
    _after: Condition,
) -> Result<()> {
    todo!()
}
