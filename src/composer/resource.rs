// Copyright 2022 The Amphitheatre Authors.
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

use k8s_openapi::apiextensions_apiserver as server;
use kube::api::{DeleteParams, PostParams};
use kube::{Api, Client, CustomResourceExt, ResourceExt};
use serde_json::to_string_pretty;
use server::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use tokio::time::sleep;
use tracing::debug;

use super::types::{Playbook, PlaybookSpec, PLAYBOOK_RESOURCE_NAME};

pub async fn install(client: Client) -> Result<(), Box<dyn std::error::Error>> {
    let api: Api<CustomResourceDefinition> = Api::all(client);
    let crd = Playbook::crd();
    debug!(
        "Creating the Playbook CustomResourceDefinition: {}",
        to_string_pretty(&crd)?
    );

    let params = PostParams::default();
    match api.create(&params, &crd).await {
        Ok(o) => {
            debug!("Created {} ({:?})", o.name_any(), o.status.unwrap());
            debug!("Created CRD: {:?}", o.spec);
        }
        Err(kube::Error::Api(err)) => assert_eq!(err.code, 409), /* if you skipped delete, for instance */
        Err(err) => return Err(err.into()),
    }

    // Wait for the api to catch up
    sleep(Duration::from_secs(1)).await;

    Ok(())
}

pub async fn uninstall(client: Client) -> Result<(), Box<dyn std::error::Error>> {
    let api: Api<CustomResourceDefinition> = Api::all(client);
    let params = DeleteParams::default();

    // Ignore delete error if not exists
    let _ = api
        .delete(PLAYBOOK_RESOURCE_NAME, &params)
        .await?
        .map_left(|o| debug!("Deleting CRD: {:?}", o.status))
        .map_right(|s| debug!("Deleted CRD: {:?}", s));

    // Wait for the delete to take place (map-left case or delete from previous run)
    sleep(Duration::from_secs(2)).await;

    Ok(())
}

pub async fn create(
    client: Client,
    namespace: String,
    name: String,
    title: String,
    description: String,
) -> Result<Playbook, kube::Error> {
    let api: Api<Playbook> = Api::namespaced(client, namespace.as_str());
    let params = PostParams::default();

    let playbook = Playbook::new(
        name.as_str(),
        PlaybookSpec {
            title,
            description,
            actors: vec![],
        },
    );

    api.create(&params, &playbook).await
}
