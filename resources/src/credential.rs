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

use amp_common::config::Configuration;
use amp_common::docker::DockerConfig;
use k8s_openapi::api::core::v1::Secret;
use kube::{Client, ResourceExt};
use tracing::info;

use super::error::Result;
use crate::{secret, service_account};

pub async fn sync(
    client: &Client,
    namespace: &str,
    service_account_name: &str,
    configuration: &Configuration,
) -> Result<()> {
    info!("The current configuration reads: {:#?}", configuration);

    let mut secrets = vec![];

    secrets.extend(sync_registry_credentials(client, namespace, configuration).await?);
    secrets.extend(sync_repository_credentials(client, namespace, configuration).await?);

    // Patch this credentials to service account
    info!("Patch the credentials to service account");
    service_account::patch(client, namespace, service_account_name, &secrets, true, true).await?;

    Ok(())
}

/// Create Docker registry secrets.
async fn sync_registry_credentials(
    client: &Client,
    namespace: &str,
    configuration: &Configuration,
) -> Result<Vec<Secret>> {
    let mut secrets = vec![];

    let config = DockerConfig::from(&configuration.registry);
    let secret = secret::create_registry_secret(client, namespace, config).await?;

    info!("Created Secret for Docker Registry: {:#?}", secret.name_any());
    secrets.push(secret);

    Ok(secrets)
}

// Create repository secrets.
async fn sync_repository_credentials(
    client: &Client,
    namespace: &str,
    configuration: &Configuration,
) -> Result<Vec<Secret>> {
    let mut secrets = vec![];

    for (endpoint, credential) in configuration.repositories.iter() {
        let secret = secret::create_repository_secret(client, namespace, endpoint, credential).await?;
        info!("Created Secret {} for repository: {} ", secret.name_any(), endpoint);
        secrets.push(secret.clone());
    }

    Ok(secrets)
}
