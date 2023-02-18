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
use tracing::error;
use uuid::Uuid;

use crate::context::Context;
use crate::models::playbook::Playbook;
use crate::repositories::playbook::PlaybookRepository;
use crate::resources::crds::{ActorSpec, Partner, PlaybookSpec};
use crate::resources::playbook;
use crate::response::ApiError;
use crate::services::Result;

pub struct PlaybookService;

impl PlaybookService {
    pub async fn get(ctx: &State<Arc<Context>>, id: Uuid) -> Result<Option<Playbook>> {
        PlaybookRepository::get(&ctx.db, id)
            .await
            .map_err(|_| ApiError::DatabaseError)
    }

    pub async fn list(ctx: &State<Arc<Context>>) -> Result<Vec<Playbook>> {
        PlaybookRepository::list(&ctx.db).await.map_err(|err| {
            error!("{:?}", err);
            ApiError::DatabaseError
        })
    }

    pub async fn start(ctx: &State<Arc<Context>>, id: Uuid) -> Result<()> {
        PlaybookRepository::change_state(&ctx.db, id, "RUNNING")
            .await
            .map_err(|_| ApiError::DatabaseError)
    }

    pub async fn stop(ctx: &State<Arc<Context>>, id: Uuid) -> Result<()> {
        PlaybookRepository::change_state(&ctx.db, id, "STOPPED")
            .await
            .map_err(|_| ApiError::DatabaseError)
    }

    pub async fn delete(ctx: &State<Arc<Context>>, id: Uuid) -> Result<()> {
        PlaybookRepository::delete(&ctx.db, id)
            .await
            .map_err(|_| ApiError::DatabaseError)
    }

    pub async fn create(ctx: &State<Arc<Context>>, title: String, description: String) -> Result<Uuid> {
        let uuid = Uuid::new_v4();
        let namespace = format!("amp-{}", uuid);

        let spec = Self::read(ctx, title, description, namespace).await?.unwrap();
        let _playbook = playbook::create(ctx.k8s.clone(), uuid.to_string(), spec)
            .await
            .map_err(|err| {
                error!("{:?}", err);
                ApiError::KubernetesError
            })?;

        Ok(uuid)
        // PlaybookRepository::create(&ctx.db, title, description)
        //     .await
        //     .map_err(|err| {
        //         error!("{:?}", err);
        //         ApiError::DatabaseError
        //     })
    }

    pub async fn update(
        ctx: &State<Arc<Context>>,
        id: Uuid,
        title: Option<String>,
        description: Option<String>,
    ) -> Result<Playbook> {
        PlaybookRepository::update(&ctx.db, id, title, description)
            .await
            .map_err(|_| ApiError::DatabaseError)
    }

    pub async fn read(
        ctx: &Arc<Context>,
        title: String,
        description: String,
        namespace: String,
    ) -> Result<Option<PlaybookSpec>> {
        let spec = PlaybookSpec {
            title,
            description,
            namespace,
            actors: vec![ActorSpec {
                name: "amp-example-java".into(),
                description: "A simple Java example app".into(),
                image: format!("{}/{}", ctx.config.registry_namespace, "amp-example-java"),
                repository: "https://github.com/amphitheatre-app/amp-example-java.git".into(),
                commit: "875db185acc8bf7c7effc389a350cae7aa926e57".into(),
                partners: Some(vec![Partner {
                    name: "amp-example-go".into(),
                    repository: "https://github.com/amphitheatre-app/amp-example-go.git".into(),
                    ..Partner::default()
                }]),
                ..ActorSpec::default()
            }],
            ..PlaybookSpec::default()
        };

        Ok(Some(spec))
    }
}
