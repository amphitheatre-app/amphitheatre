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

use std::collections::HashMap;
use std::fmt::Display;

use k8s_openapi::api::core::v1::Secret;
use kube::api::PostParams;
use kube::{Api, Client};
use serde_json::{from_value, json};

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
        format!("{}-{}-secret", self.kind, self.location)
    }
}

pub async fn create(client: Client, namespace: &str, credential: &Credential) -> Result<Secret> {
    let api: Api<Secret> = Api::namespaced(client, namespace);
    let params = PostParams::default();

    let annotations = HashMap::from([(credential.kind.schema_name(), credential.location.clone())]);

    let data = match credential.auth {
        Authorization::Basic => HashMap::from([
            ("username", credential.username.clone()),
            ("password", credential.password.clone()),
        ]),
        Authorization::Token => HashMap::from([("value", credential.token.clone())]),
    };

    let mut secret = from_value(json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "metadata": {
            "name": credential.name(),
            "annotations": annotations,
        },
        "type": credential.auth.schema_name(),
        "data": data
    }))
    .map_err(Error::SerializationError)?;

    tracing::info!(
        "created secret resource: {:#?}",
        serde_yaml::to_string(&secret)
    );

    secret = api
        .create(&params, &secret)
        .await
        .map_err(Error::KubeError)?;

    Ok(secret)
}
