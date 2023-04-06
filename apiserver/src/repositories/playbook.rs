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

use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use uuid::Uuid;

use crate::database::{Database, Result};
use crate::models::playbook::{ActiveModel, Entity, Playbook};

pub struct PlaybookRepository;

impl PlaybookRepository {
    pub async fn get(db: &Database, id: Uuid) -> Result<Option<Playbook>> {
        Entity::find_by_id(id).one(db).await
    }

    pub async fn list(db: &Database) -> Result<Vec<Playbook>> {
        Entity::find().all(db).await
    }

    pub async fn change_state(db: &Database, id: Uuid, state: &str) -> Result<()> {
        let playbook = Entity::find_by_id(id).one(db).await?;
        let mut playbook: ActiveModel = playbook.unwrap().into();

        playbook.state = Set(state.to_owned());
        playbook.update(db).await?;

        Ok(())
    }

    pub async fn delete(db: &Database, id: Uuid) -> Result<()> {
        let playbook = Entity::find_by_id(id).one(db).await?;
        let playbook: ActiveModel = playbook.unwrap().into();

        Entity::delete(playbook).exec(db).await?;

        Ok(())
    }

    pub async fn create(db: &Database, title: String, description: String) -> Result<Playbook> {
        let playbook = ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            title: Set(title),
            description: Set(description),
            ..Default::default()
        };
        playbook.insert(db).await
    }

    pub async fn update(
        db: &Database,
        id: Uuid,
        title: Option<String>,
        description: Option<String>,
    ) -> Result<Playbook> {
        let playbook = Entity::find_by_id(id).one(db).await?;
        let mut playbook: ActiveModel = playbook.unwrap().into();

        if title.is_some() {
            playbook.title = Set(title.unwrap());
        }

        if description.is_some() {
            playbook.description = Set(description.unwrap());
        }

        playbook.update(db).await
    }
}
