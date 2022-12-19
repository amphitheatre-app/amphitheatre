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

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "amphitheatre.app",
    version = "v1",
    kind = "Playbook",
    namespaced
)]
#[kube(status = "PlaybookStatus")]
pub struct PlaybookSpec {
    name: String,
    description: String,
    actors: Vec<Actor>,
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
    name: String,
    version: String,
    image: String,
    source: String,
    checksum: String,
    environment: HashMap<String, String>,
    labels: HashMap<String, String>,
    partners: Vec<String>,
}
