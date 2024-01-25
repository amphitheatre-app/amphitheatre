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

use amp_common::config::Credentials;
use amp_common::docker::DockerConfig;
use k8s_openapi::api::core::v1::Secret;
use kube::{Client, ResourceExt};
use tracing::{debug, info};

use super::error::Result;
use crate::{secret, service_account};

pub async fn sync(client: &Client, namespace: &str, name: &str, credentials: &Credentials) -> Result<()> {
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

/// Load the credentials from the Kubernetes secret.
/// FIXME: return the error instead of None
pub async fn load(client: &Client, namespace: &str) -> Result<Option<Credentials>> {
    let result = secret::get_opt(client, namespace, "amp-credentials").await?;
    if result.is_none() {
        debug!("the amp-credentials was not found.");
        return Ok(None);
    }
    let secret = result.unwrap();
    if secret.data.is_none() {
        debug!("the amp-credentials does not contain any data.");
        return Ok(None);
    }
    let data = secret.data.unwrap();
    if !data.contains_key("credentials") {
        debug!("the amp-credentials does not contain the credentials key.");
        return Ok(None);
    }

    let bytes = data.get("credentials").unwrap();
    if let Ok(content) = String::from_utf8(bytes.0.clone()) {
        if let Ok(credentials) = toml::from_str::<Credentials>(&content) {
            info!("Loaded the credentials from the Kubernetes secret.");
            return Ok(Some(credentials));
        }
        debug!("The credentials is not a valid TOML sequence");
    } else {
        debug!("The credentials is not a valid UTF-8 sequence");
    }

    Ok(None)
}
