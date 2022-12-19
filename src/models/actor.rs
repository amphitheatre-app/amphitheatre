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

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, ToSchema, DeriveEntityModel, Debug)]
#[sea_orm(table_name = "actors")]
pub struct Model {
    /// The id of the actor.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Which playbook belongs to.
    pub playbook_id: Uuid,
    /// The title of the actor.
    pub name: String,
    /// The description of the actor.
    pub description: String,
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

    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub type Actor = Model;
