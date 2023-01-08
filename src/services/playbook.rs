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
use serde_json::to_string_pretty;
use tracing::{error, info};
use uuid::Uuid;

use crate::app::Context;
use crate::models::playbook::Playbook;
use crate::repositories::playbook::PlaybookRepository;
use crate::resources::secret::{Credential, Kind};
use crate::resources::{namespace, playbook, secret, service_account};
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

    async fn init(ctx: &State<Arc<Context>>, namespace: &str) -> Result<()> {
        // Create namespace for this playbook
        namespace::create(ctx.k8s.clone(), namespace)
            .await
            .map_err(|err| {
                error!("Create namespace {} failed: {:?}", namespace, err);
                ApiError::KubernetesError
            })?;

        // Docker registry Credential
        let credential = Credential::basic(
            Kind::Image,
            "harbor.amp-system.svc.cluster.local".into(),
            "admin".into(),
            "Harbor12345".into(),
        );
        secret::create(ctx.k8s.clone(), namespace, &credential)
            .await
            .map_err(|err| {
                error!("Create registry credential failed: {:?}", err);
                ApiError::KubernetesError
            })?;

        // Patch this credential to default service account
        service_account::patch(
            ctx.k8s.clone(),
            namespace,
            "default",
            &credential,
            true,
            true,
        )
        .await
        .map_err(|err| {
            error!("Patch credentials to service account failed: {:?}", err);
            ApiError::KubernetesError
        })?;

        Ok(())
    }

    pub async fn create(
        ctx: &State<Arc<Context>>,
        title: String,
        description: String,
    ) -> Result<Uuid> {
        let uuid = Uuid::new_v4();
        let namespace = format!("amp-{}", uuid);

        // Init create namespace, credentials and service accounts
        Self::init(ctx, namespace.as_str()).await?;

        let playbook = playbook::create(
            ctx.k8s.clone(),
            namespace.as_str(),
            uuid.to_string(),
            title,
            description,
        )
        .await
        .map_err(|err| {
            error!("{:?}", err);
            ApiError::KubernetesError
        })?;

        info!(
            "Creating the playbook: {}",
            to_string_pretty(&playbook).unwrap()
        );

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
}
