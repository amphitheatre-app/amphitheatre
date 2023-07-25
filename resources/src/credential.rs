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

use amp_common::config::Credentials;
use amp_common::docker::DockerConfig;
use k8s_openapi::api::core::v1::Secret;
use kube::{Client, ResourceExt};
use tracing::{debug, info};

use super::error::Result;
use crate::{secret, service_account};

pub async fn sync(client: &Client, namespace: &str, name: &str, credentials: &Credentials) -> Result<()> {
    debug!("The current configuration reads: {:?}", credentials);

    let mut secrets = vec![];

    // Patch the image pull secrets to service account
    info!("Patch the image pull secrets to Service Account {}", name);
    secrets.extend(sync_registry_credentials(client, namespace, credentials).await?);
    service_account::patch(client, namespace, name, &secrets, false, true).await?;

    // Patch the secrets to service account
    info!("Patch the secrets to Service Account {}", name);
    secrets.extend(sync_repository_credentials(client, namespace, credentials).await?);
    service_account::patch(client, namespace, name, &secrets, true, false).await?;

    // @TODO: Clean up unused secrets

    Ok(())
}

/// Sync Docker registry credentials.
async fn sync_registry_credentials(client: &Client, namespace: &str, credentials: &Credentials) -> Result<Vec<Secret>> {
    let mut secrets = vec![];

    let config = DockerConfig::from(&credentials.registries);
    let secret = secret::create_registry_secret(client, namespace, config).await?;

    info!("Created Secret {} for Docker Registries", secret.name_any());
    secrets.push(secret);

    Ok(secrets)
}

// Sync repository credentials.
async fn sync_repository_credentials(
    client: &Client,
    namespace: &str,
    credentials: &Credentials,
) -> Result<Vec<Secret>> {
    let mut secrets = vec![];

    if let Some(repositories) = &credentials.repositories {
        for credential in repositories.iter() {
            let endpoint = &credential.server;
            let secret = secret::create_repository_secret(client, namespace, endpoint, credential).await?;
            info!("Created Secret {} for repository: {} ", secret.name_any(), endpoint);
            secrets.push(secret.clone());
        }
    }

    Ok(secrets)
}
