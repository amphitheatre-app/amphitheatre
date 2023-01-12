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

use std::fmt::Display;

use convert_case::{Case, Casing};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
use k8s_openapi::chrono::Utc;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

use self::PlaybookState::*;
use super::actor::ActorSpec;

pub static PLAYBOOK_RESOURCE_NAME: &str = "playbooks.amphitheatre.app";

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Validate)]
#[kube(
    group = "amphitheatre.app",
    version = "v1",
    kind = "Playbook",
    status = "PlaybookStatus"
)]
pub struct PlaybookSpec {
    pub title: String,
    pub description: String,
    pub namespace: String,

    #[validate(length(min = 1))]
    pub actors: Vec<ActorSpec>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub struct PlaybookStatus {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    conditions: Vec<Condition>,
}

impl PlaybookStatus {
    pub fn pending(&self) -> bool {
        self.state(Pending, true)
    }

    pub fn solving(&self) -> bool {
        self.state(Solving, true)
    }

    pub fn ready(&self) -> bool {
        self.state(Ready, true)
    }

    pub fn running(&self) -> bool {
        self.state(Running, true)
    }

    pub fn succeeded(&self) -> bool {
        self.state(Succeeded, true)
    }

    pub fn failed(&self) -> bool {
        self.state(Failed, true)
    }

    fn state(&self, s: PlaybookState, status: bool) -> bool {
        self.conditions.iter().any(|condition| {
            condition.type_ == s.to_string()
                && condition.status == status.to_string().to_case(Case::Pascal)
        })
    }
}

pub enum PlaybookState {
    Pending,
    Solving,
    Ready,
    Running,
    Succeeded,
    Failed,
}
impl PlaybookState {
    pub fn pending() -> Condition {
        PlaybookState::create(Pending, true, "Created", None)
    }

    pub fn solving() -> Condition {
        PlaybookState::create(Solving, true, "Solve", None)
    }

    pub fn ready() -> Condition {
        PlaybookState::create(Ready, true, "Solved", None)
    }

    pub fn running(status: bool, reason: &str, message: Option<String>) -> Condition {
        PlaybookState::create(Running, status, reason, message)
    }

    pub fn succeeded(status: bool, reason: &str, message: Option<String>) -> Condition {
        PlaybookState::create(Succeeded, status, reason, message)
    }

    pub fn failed(status: bool, reason: &str, message: Option<String>) -> Condition {
        PlaybookState::create(Failed, status, reason, message)
    }

    #[inline]
    fn create(
        state: PlaybookState,
        status: bool,
        reason: &str,
        message: Option<String>,
    ) -> Condition {
        Condition {
            type_: state.to_string(),
            status: status.to_string().to_case(Case::Pascal),
            last_transition_time: Time(Utc::now()),
            reason: reason.to_case(Case::Pascal),
            observed_generation: None,
            message: match message {
                Some(message) => message,
                None => "".to_string(),
            },
        }
    }
}

impl Display for PlaybookState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pending => f.write_str("Pending"),
            Solving => f.write_str("Solving"),
            Ready => f.write_str("Ready"),
            Running => f.write_str("Running"),
            Succeeded => f.write_str("Succeeded"),
            Failed => f.write_str("Failed"),
        }
    }
}
