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

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub static ACTOR_RESOURCE_NAME: &str = "actors.amphitheatre.app";

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Validate)]
#[kube(
    group = "amphitheatre.app",
    version = "v1",
    kind = "Actor",
    status = "ActorStatus",
    namespaced
)]
pub struct ActorSpec {
    /// The title of the actor.
    pub name: String,
    /// The description of the actor.
    pub description: String,
    /// Specifies the image to launch the container. The image must follow
    /// the Open Container Specification addressable image format.
    /// such as: [<registry>/][<project>/]<image>[:<tag>|@<digest>].
    pub image: String,
    /// Git repository the package should be cloned from.
    /// e.g. https://github.com/amphitheatre-app/amphitheatre.git.
    pub repo: String,
    /// Relative path from the repo root to the configuration file.
    /// eg. getting-started/amp.yaml.
    pub path: String,
    /// Git ref the package should be cloned from. eg. master or main
    pub reference: String,
    /// The selected commit of the actor.
    pub commit: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub partners: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<Service>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Service {
    pub kind: String,
    pub ports: Vec<Port>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Port {
    pub port: u32,
    pub protocol: String,
    pub expose: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum ActorStatus {
    Pending,
    Building,
    Running,
    Failed,
}

impl Display for ActorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorStatus::Pending => f.write_str("Pending"),
            ActorStatus::Building => f.write_str("Building"),
            ActorStatus::Running => f.write_str("Running"),
            ActorStatus::Failed => f.write_str("Failed"),
        }
    }
}
