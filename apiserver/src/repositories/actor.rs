// Copyright 2023 The Amphitheatre Authors.
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

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::database::{Database, Result};
use crate::models::actor::{Actor, Column, Entity};

pub struct ActorRepository;

impl ActorRepository {
    pub async fn get(db: &Database, id: Uuid) -> Result<Option<Actor>> {
        Entity::find_by_id(id).one(db).await
    }

    pub async fn list(db: &Database, pid: Uuid) -> Result<Vec<Actor>> {
        Entity::find().filter(Column::PlaybookId.eq(pid)).all(db).await
    }
}
