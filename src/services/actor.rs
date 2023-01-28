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
use uuid::Uuid;

use crate::context::Context;
use crate::models::actor::Actor;
use crate::repositories::actor::ActorRepository;
use crate::resources::crds::{ActorSpec, Partner};
use crate::response::ApiError;
use crate::services::Result;

pub struct ActorService;

impl ActorService {
    pub async fn get(ctx: &State<Arc<Context>>, id: Uuid) -> Result<Option<Actor>> {
        ActorRepository::get(&ctx.db, id)
            .await
            .map_err(|_| ApiError::DatabaseError)
    }

    pub async fn list(ctx: &State<Arc<Context>>, pid: Uuid) -> Result<Vec<Actor>> {
        ActorRepository::list(&ctx.db, pid)
            .await
            .map_err(|_| ApiError::DatabaseError)
    }

    // TODO: Read real actor information from remote VCS (like github).
    pub async fn read(ctx: &Arc<Context>, partner: &Partner) -> Result<Option<ActorSpec>> {
        let spec = ActorSpec {
            name: partner.name.clone(),
            description: "A simple NodeJs example app".into(),
            image: format!("{}/{}", ctx.config.registry_namespace, "amp-example-nodejs"),
            repository: partner.repository.clone(),
            reference: partner.reference.clone(),
            path: partner.path.clone(),
            commit: "285ef2bc98fb6b3db46a96b6a750fad2d0c566b5".into(),
            ..ActorSpec::default()
        };

        Ok(Some(spec))
    }
}
