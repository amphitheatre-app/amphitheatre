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

use std::collections::HashSet;

use k8s_openapi::api::core::v1::{LocalObjectReference, ObjectReference, Secret, ServiceAccount};
use kube::api::{Patch, PatchParams};
use kube::{Api, Client, ResourceExt};
use serde_json::json;
use tracing::debug;

use super::error::{Error, Result};

pub async fn patch(
    client: &Client,
    namespace: &str,
    name: &str,
    new_secrets: &Vec<Secret>,
    append_to_secret: bool,
    append_to_image_pull_secret: bool,
) -> Result<ServiceAccount> {
    let api: Api<ServiceAccount> = Api::namespaced(client.clone(), namespace);
    let mut account = api.get(name).await.map_err(Error::KubeError)?;

    debug!("The current Service Account {} is: {:?}", name, account);

    // Fetch original secrets and image pull secrets.
    let mut secrets = account.secrets.map_or(vec![], |v| v);
    let secret_names: HashSet<String> = HashSet::from_iter(secrets.iter().map(|s| s.name.clone().unwrap()));

    let mut image_pull_secrets = account.image_pull_secrets.map_or(vec![], |v| v);
    let image_pull_secret_names: HashSet<String> =
        HashSet::from_iter(image_pull_secrets.iter().map(|s| s.name.clone().unwrap()));

    for secret in new_secrets {
        let secret_name = secret.name_any();

        // Append to original secrets.
        if append_to_secret && !secret_names.contains(&secret_name) {
            secrets.push(ObjectReference {
                name: Some(secret_name.clone()),
                ..Default::default()
            });
        }

        // Append to original image pull secrets.
        if append_to_image_pull_secret && !image_pull_secret_names.contains(&secret_name) {
            image_pull_secrets.push(LocalObjectReference {
                name: Some(secret_name.clone()),
            });
        }
    }

    // Create patch for update.
    let json = json!({"secrets": secrets, "imagePullSecrets": image_pull_secrets });
    debug!("The patch of Service Account {} is: {:?}", name, json);

    // Save to Kubernetes cluster
    let param = PatchParams::apply("amp-controllers");
    let patch = Patch::Strategic(&json);
    account = api.patch(name, &param, &patch).await.map_err(Error::KubeError)?;

    debug!("The final Service Account {} is {:?}", name, account);
    Ok(account)
}
