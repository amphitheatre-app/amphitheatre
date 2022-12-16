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

use std::sync::Arc;

use axum::extract::State;

use crate::app::Context;
use crate::models::actor::Actor;
use crate::repositories::actor::ActorRepository;
use crate::response::ApiError;
use crate::services::Result;

pub struct ActorService;

impl ActorService {
    pub async fn get(ctx: &State<Arc<Context>>, id: u64) -> Result<Option<Actor>> {
        ActorRepository::get(&ctx.db, id)
            .await
            .map_err(|_| ApiError::DatabaseError)
    }

    pub async fn list(ctx: &State<Arc<Context>>, pid: u64) -> Result<Vec<Actor>> {
        ActorRepository::list(&ctx.db, pid)
            .await
            .map_err(|_| ApiError::DatabaseError)
    }
}
