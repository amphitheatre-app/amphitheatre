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

use rocket::serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[serde(crate = "rocket::serde")]
#[table_name = "players"]

pub struct Player {
    pub id: u64,

    // An identifier for the project.
    pub name: String,

    // Git repository the package should be cloned from.
    // e.g. https://github.com/amphitheatre-app/amphitheatre.git.
    pub repo: String,

    // Relative path from the repo root to the configuration file.
    // eg. getting-started/amp.yaml.
    pub path: String,

    // Git ref the package should be cloned from. eg. master or main
    pub reference: String,

    pub commit: String,
}

table! {
    players(id) {
        id -> Unsigned<BigInt>,
        name -> Text,
        repo -> Text,
        path -> Text,
        reference -> Text,
        commit -> Text,
    }
}
