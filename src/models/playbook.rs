// Copyright 2022 The Amphitheatre Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Playbook {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub state: String,

    pub created_at: String,
    pub updated_at: String,
}

impl Default for Playbook {
    fn default() -> Self {
        Self {
            id: 123,
            title: "consectetur occaecat consectetur exercitation fugiat".into(),
            description: "laboris esse quis adipisicing sunt eu enim ipsum".into(),
            state: "RUNNING".into(),
            created_at: "2022-12-05T040000Z".into(),
            updated_at: "2022-12-05T040000Z".into(),
        }
    }
}
