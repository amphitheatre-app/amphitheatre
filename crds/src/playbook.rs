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

use super::actor::ActorSpec;

#[derive(CustomResource, Default, Deserialize, Serialize, Clone, Debug, JsonSchema, Validate)]
#[kube(
    group = "amphitheatre.app",
    version = "v1",
    kind = "Playbook",
    status = "PlaybookStatus"
)]
pub struct PlaybookSpec {
    /// The title of the playbook
    pub title: String,
    /// The description of the playbook
    pub description: String,
    /// Namespace for its managed resources and Actor deployment instances
    pub namespace: String,
    /// Global sync mode, if enabled, pulls the latest code from source version
    /// control in real time via Webhook, etc. and then rebuilds and deploys it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync: Option<bool>,
    /// All the actors involved in this playbook
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
        self.state(PlaybookState::Pending, true)
    }

    pub fn solving(&self) -> bool {
        self.state(PlaybookState::Solving, true)
    }

    pub fn ready(&self) -> bool {
        self.state(PlaybookState::Ready, true)
    }

    pub fn running(&self) -> bool {
        self.state(PlaybookState::Running, true)
    }

    pub fn succeeded(&self) -> bool {
        self.state(PlaybookState::Succeeded, true)
    }

    pub fn failed(&self) -> bool {
        self.state(PlaybookState::Failed, true)
    }

    fn state(&self, s: PlaybookState, status: bool) -> bool {
        self.conditions.iter().any(|condition| {
            condition.type_ == s.to_string() && condition.status == status.to_string().to_case(Case::Pascal)
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
        PlaybookState::create(PlaybookState::Pending, true, "Created", None)
    }

    pub fn solving() -> Condition {
        PlaybookState::create(PlaybookState::Solving, true, "Solve", None)
    }

    pub fn ready() -> Condition {
        PlaybookState::create(PlaybookState::Ready, true, "Solved", None)
    }

    pub fn running(status: bool, reason: &str, message: Option<String>) -> Condition {
        PlaybookState::create(PlaybookState::Running, status, reason, message)
    }

    pub fn succeeded(status: bool, reason: &str, message: Option<String>) -> Condition {
        PlaybookState::create(PlaybookState::Succeeded, status, reason, message)
    }

    pub fn failed(status: bool, reason: &str, message: Option<String>) -> Condition {
        PlaybookState::create(PlaybookState::Failed, status, reason, message)
    }

    #[inline]
    fn create(state: PlaybookState, status: bool, reason: &str, message: Option<String>) -> Condition {
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
            PlaybookState::Pending => f.write_str("Pending"),
            PlaybookState::Solving => f.write_str("Solving"),
            PlaybookState::Ready => f.write_str("Ready"),
            PlaybookState::Running => f.write_str("Running"),
            PlaybookState::Succeeded => f.write_str("Succeeded"),
            PlaybookState::Failed => f.write_str("Failed"),
        }
    }
}
