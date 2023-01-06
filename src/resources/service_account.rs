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

use k8s_openapi::api::core::v1::{LocalObjectReference, ObjectReference, ServiceAccount};
use kube::api::{Patch, PatchParams};
use kube::{Api, Client};
use serde_json::json;

use super::error::Result;
use super::secret::Credential;
use crate::resources::error::Error;

pub async fn patch(
    client: Client,
    namespace: &str,
    service_account_name: &str,
    credential: &Credential,
    secret: bool,
    image_pull_secret: bool,
) -> Result<ServiceAccount> {
    let api: Api<ServiceAccount> = Api::namespaced(client, namespace);
    let mut service_account = api
        .get(service_account_name)
        .await
        .map_err(Error::KubeError)?;

    let secret_name = credential.name();

    // Fetch original secrets
    let mut secrets = service_account.secrets.map_or(vec![], |v| v);
    if secret {
        secrets.push(ObjectReference {
            name: Some(secret_name.clone()),
            ..Default::default()
        });
    }

    // Fetch original imagePullSecrets
    let mut image_pull_secrets = service_account.image_pull_secrets.map_or(vec![], |v| v);
    if image_pull_secret {
        image_pull_secrets.push(LocalObjectReference {
            name: Some(secret_name.clone()),
        });
    }

    // Create patch for update.
    let patch = json!({"spec": { "secrets": secrets, "imagePullSecrets": image_pull_secrets }});
    service_account = api
        .patch(
            service_account_name,
            &PatchParams::apply("amp-composer"),
            &Patch::Merge(&patch),
        )
        .await
        .map_err(Error::KubeError)?;

    tracing::info!(
        "Added Secret {:?} for ServiceAccount {}",
        secret_name,
        service_account_name
    );

    Ok(service_account)
}
