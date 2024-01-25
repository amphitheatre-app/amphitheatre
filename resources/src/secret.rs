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

use amp_common::config::{Credential, Scheme};
use amp_common::docker::DockerConfig;
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::ByteString;
use kube::api::{Patch, PatchParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, ResourceExt};
use tracing::{debug, info};
use url::Url;

use super::error::{Error, Result};

pub async fn create_registry_secret(client: &Client, namespace: &str, config: DockerConfig) -> Result<Secret> {
    let resource = Secret {
        metadata: ObjectMeta { name: Some("amp-registry-credentials".to_string()), ..Default::default() },
        type_: Some("kubernetes.io/dockerconfigjson".to_string()),
        data: Some(BTreeMap::from([(
            ".dockerconfigjson".to_string(),
            ByteString(serde_json::to_vec(&config).map_err(Error::SerializationError).unwrap()),
        )])),
        ..Default::default()
    };
    debug!("Created registry secret: {}", resource.name_any());

    create(client, namespace, resource).await
}

pub async fn create_repository_secret(
    client: &Client,
    namespace: &str,
    endpoint: &str,
    credential: &impl Credential,
) -> Result<Secret> {
    let mut secret_type = String::from("Opaque");
    let mut data = BTreeMap::new();

    match credential.scheme() {
        Scheme::Basic => {
            secret_type = String::from("kubernetes.io/basic-auth");
            data = BTreeMap::from([
                ("username".to_string(), credential.username_any()),
                ("password".to_string(), credential.password_any()),
            ]);
        }
        Scheme::Bearer => {
            secret_type = String::from("kubernetes.io/ssh-auth");
            data = BTreeMap::from([("ssh-privatekey".to_string(), credential.token_any())]);
        }
        Scheme::Unknown => {}
    }

    let resource = Secret {
        metadata: ObjectMeta { name: Some(secret_name(endpoint)?), ..ObjectMeta::default() },
        type_: Some(secret_type),
        string_data: Some(data),
        ..Secret::default()
    };
    debug!("Created repository secret: {}", resource.name_any());

    create(client, namespace, resource).await
}

fn secret_name(endpoint: &str) -> Result<String> {
    let location = Url::parse(endpoint).map_err(Error::UrlParseError)?;
    let name = format!("amp-repo-credentials-{}-{}", location.scheme(), location.host_str().unwrap(),).to_lowercase();

    Ok(name)
}

pub async fn create(client: &Client, namespace: &str, resource: Secret) -> Result<Secret> {
    let api: Api<Secret> = Api::namespaced(client.clone(), namespace);
    let name = resource.name_any();

    let params = &PatchParams::apply("amp-controllers").force();
    let secret = api.patch(&name, params, &Patch::Apply(&resource)).await.map_err(Error::KubeError)?;

    info!("Added Secret {:?}", name);
    Ok(secret)
}

/// Get a secret by name
pub async fn get_opt(client: &Client, namespace: &str, name: &str) -> Result<Option<Secret>> {
    let api: Api<Secret> = Api::namespaced(client.clone(), namespace);
    let resources = api.get_opt(name).await.map_err(Error::KubeError)?;

    Ok(resources)
}
