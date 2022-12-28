// Copyright 2022 The Amphitheatre Authors.
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

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub static PLAYBOOK_RESOURCE_NAME: &str = "playbooks.amphitheatre.app";

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Validate)]
#[kube(
    group = "amphitheatre.app",
    version = "v1",
    kind = "Playbook",
    namespaced
)]
#[kube(status = "PlaybookStatus")]
pub struct PlaybookSpec {
    pub title: String,
    pub description: String,
    #[validate(length(min = 1))]
    pub actors: Vec<Actor>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum PlaybookStatus {
    Initial,
    Completed,
    Downloaded,
    Builded,
    Published,
    Deployed,
    Finished,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct Actor {
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

    pub environment: HashMap<String, String>,
    pub partners: Vec<String>,
}
