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

use amp_common::schema::Actor;
use kube::ResourceExt;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ActorResponse {
    /// The actor ID in Amphitheatre.
    pub id: String,
    /// The title of the actor.
    pub title: String,
    /// The description of the actor.
    pub description: String,
}

impl From<&Actor> for ActorResponse {
    fn from(value: &Actor) -> Self {
        Self {
            id: value.name_any(),
            title: value.spec.name.clone(),
            description: value.spec.description.clone().unwrap_or_default(),
        }
    }
}
