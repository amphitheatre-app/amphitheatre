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

use k8s_openapi::api::core::v1::{LocalObjectReference, ObjectReference, Secret, ServiceAccount};
use kube::api::{Patch, PatchParams};
use kube::{Api, Client, ResourceExt};
use serde_json::json;

use super::error::{Error, Result};

pub async fn patch(
    client: Client,
    namespace: &str,
    service_account_name: &str,
    new_secrets: &Vec<Secret>,
    append_to_secret: bool,
    append_to_image_pull_secret: bool,
) -> Result<ServiceAccount> {
    let api: Api<ServiceAccount> = Api::namespaced(client, namespace);
    let mut service_account = api.get(service_account_name).await.map_err(Error::KubeError)?;

    tracing::debug!(
        "The current {} ServiceAccount is: {:#?}",
        service_account_name,
        service_account
    );

    // Fetch original secrets and image pull secrets.
    let mut secrets = service_account.secrets.map_or(vec![], |v| v);
    let mut image_pull_secrets = service_account.image_pull_secrets.map_or(vec![], |v| v);

    for secret in new_secrets {
        let secret_name = secret.name_any();

        // Append to original secrets.
        if append_to_secret {
            secrets.push(ObjectReference {
                name: Some(secret_name.clone()),
                ..Default::default()
            });
        }

        // Append to original image pull secrets.
        if append_to_image_pull_secret {
            image_pull_secrets.push(LocalObjectReference {
                name: Some(secret_name.clone()),
            });
        }
    }

    tracing::debug!("The secrets is: {:#?}", secrets);
    tracing::debug!("The image_pull_secrets is: {:#?}", image_pull_secrets);

    // Create patch for update.
    let patch = json!({"secrets": secrets, "imagePullSecrets": image_pull_secrets });
    tracing::debug!("The service account patch is: {:#?}", patch);

    // Save to Kubernetes cluster
    service_account = api
        .patch(
            service_account_name,
            &PatchParams::apply("amp-controllers"),
            &Patch::Merge(&patch),
        )
        .await
        .map_err(Error::KubeError)?;

    tracing::debug!(
        "Added Secrets {:?} for ServiceAccount {}: {:#?}",
        secrets,
        service_account_name,
        service_account
    );

    Ok(service_account)
}
