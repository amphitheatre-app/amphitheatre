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
#[sea_orm(table_name = "playbooks")]
pub struct Model {
    /// The id of the playbook
    #[sea_orm(primary_key)]
    pub id: u64,
    /// The title of the playbook
    pub title: String,
    /// The description of the playbook
    pub description: String,
    /// The state of the playbook
    pub state: String,

    #[schema(value_type = String)]
    pub created_at: DateTime,

    #[schema(value_type = String)]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub type Playbook = Model;
