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

use std::collections::BTreeMap;
use std::fmt::Display;

use k8s_openapi::api::core::v1::Secret;
use kube::api::{Patch, PatchParams};
use kube::core::ObjectMeta;
use kube::{Api, Client, ResourceExt};
use serde_json::to_string;

use super::error::{Error, Result};

pub enum Kind {
    Image,
    Source,
}

impl Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Image => f.write_str("Image"),
            Kind::Source => f.write_str("Source"),
        }
    }
}

impl Kind {
    pub fn schema_name(&self) -> String {
        match self {
            Kind::Image => String::from("kpack.io/docker"),
            Kind::Source => String::from("kpack.io/git"),
        }
    }
}

pub enum Authorization {
    Basic,
    Token,
}

impl Authorization {
    pub fn schema_name(&self) -> String {
        match self {
            Authorization::Basic => String::from("kubernetes.io/basic-auth"),
            Authorization::Token => String::from("kubernetes.io/ssh-auth"),
        }
    }
}

// Image,  Location, Basic, Username, Password
// Source, Location, Basic, Username, Password
// Source, Location, SSH, Token
pub struct Credential {
    pub kind: Kind,
    auth: Authorization,
    location: String,
    username: Option<String>,
    password: Option<String>,
    token: Option<String>,
}

impl Credential {
    pub fn basic(kind: Kind, location: String, username: String, password: String) -> Self {
        Self {
            kind,
            auth: Authorization::Basic,
            location,
            username: Some(username),
            password: Some(password),
            token: None,
        }
    }

    pub fn token(kind: Kind, location: String, token: String) -> Self {
        Self {
            kind,
            auth: Authorization::Token,
            location,
            username: None,
            password: None,
            token: Some(token),
        }
    }

    pub fn name(&self) -> String {
        format!("{}-{}-secret", self.kind, self.location).to_lowercase()
    }

    pub fn username_any(&self) -> String {
        self.username.clone().unwrap_or_default()
    }

    pub fn password_any(&self) -> String {
        self.password.clone().unwrap_or_default()
    }

    pub fn token_any(&self) -> String {
        self.token.clone().unwrap_or_default()
    }
}

pub async fn create(client: Client, namespace: String, credential: &Credential) -> Result<Secret> {
    let api: Api<Secret> = Api::namespaced(client, &namespace);

    let annotations =
        BTreeMap::from([(credential.kind.schema_name(), credential.location.clone())]);

    let data = match credential.auth {
        Authorization::Basic => BTreeMap::from([
            ("username".to_string(), credential.username_any()),
            ("password".to_string(), credential.password_any()),
        ]),
        Authorization::Token => {
            BTreeMap::from([("ssh-privatekey".to_string(), credential.token_any())])
        }
    };

    let resource = Secret {
        metadata: ObjectMeta {
            name: Some(credential.name()),
            annotations: Some(annotations),
            ..ObjectMeta::default()
        },
        type_: Some(credential.auth.schema_name()),
        string_data: Some(data),
        ..Secret::default()
    };
    tracing::debug!("The secret resource:\n {:#?}\n", to_string(&resource));

    let secret = api
        .patch(
            &credential.name(),
            &PatchParams::apply("amp-composer").force(),
            &Patch::Apply(&resource),
        )
        .await
        .map_err(Error::KubeError)?;

    tracing::info!("Added Secret {:?}", secret.name_any());
    Ok(secret)
}
