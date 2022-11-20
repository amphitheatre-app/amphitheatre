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

use diesel::QueryResult;

use crate::database::Database;
use crate::models::playbook::Playbook;
use crate::repositories::playbook::PlaybookRepository;

pub struct PlaybookService;

impl PlaybookService {
    pub async fn get(db: &Database, id: u64) -> QueryResult<Playbook> {
        PlaybookRepository::get(&db, id).await
    }

    pub async fn list(db: &Database) -> QueryResult<Vec<Playbook>> {
        PlaybookRepository::list(&db).await
    }
}
