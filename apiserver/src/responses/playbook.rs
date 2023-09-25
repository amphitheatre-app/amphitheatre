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

use amp_common::resource::Playbook;
use chrono::{DateTime, Utc};
use kube::ResourceExt;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PlaybookResponse {
    /// The playbook ID in Amphitheatre.
    pub id: String,
    /// The title of the playbook.
    pub title: String,
    /// The description of the playbook.
    pub description: String,
    /// When the playbook was created in Amphitheatre.
    pub created_at: DateTime<Utc>,
    /// When the playbook was last updated in Amphitheatre.
    pub updated_at: DateTime<Utc>,
}

impl From<Playbook> for PlaybookResponse {
    fn from(playbook: Playbook) -> Self {
        Self {
            id: playbook.name_any(),
            title: playbook.spec.title,
            description: playbook.spec.description.unwrap_or_default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
