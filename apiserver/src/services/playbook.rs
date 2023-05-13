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

use std::sync::Arc;

use amp_common::schema::{Playbook as PlaybookResource, PlaybookSpec};
use amp_resources::playbook;
use chrono::Utc;
use kube::ResourceExt;
use tracing::error;
use uuid::Uuid;

use crate::context::Context;
use crate::requests::playbook::CreatePlaybookRequest;
use crate::response::ApiError;
use crate::responses::playbook::PlaybookResponse;
use crate::services::Result;

pub struct PlaybookService;

impl PlaybookService {
    pub async fn get(ctx: Arc<Context>, id: Uuid) -> Result<PlaybookResponse> {
        let resource = playbook::get(&ctx.k8s, &id.to_string()).await.map_err(|err| {
            error!("{:?}", err);
            ApiError::KubernetesError
        })?;

        Ok(resource.into())
    }

    pub async fn list(ctx: Arc<Context>) -> Result<Vec<PlaybookResponse>> {
        let resources = playbook::list(&ctx.k8s).await.map_err(|err| {
            error!("{:?}", err);
            ApiError::KubernetesError
        })?;

        Ok(resources.iter().map(|playbook| playbook.to_owned().into()).collect())
    }

    pub async fn start(_ctx: Arc<Context>, _id: Uuid) -> Result<()> {
        unimplemented!()
    }

    pub async fn stop(_ctx: Arc<Context>, _id: Uuid) -> Result<()> {
        unimplemented!()
    }

    pub async fn delete(_ctx: Arc<Context>, _id: Uuid) -> Result<()> {
        unimplemented!()
    }

    pub async fn create(ctx: Arc<Context>, req: &CreatePlaybookRequest) -> Result<PlaybookResponse> {
        let uuid = Uuid::new_v4();
        let resource = PlaybookResource::new(
            &uuid.to_string(),
            PlaybookSpec {
                title: req.title.to_string(),
                description: req.description.to_string(),
                namespace: format!("amp-{}", uuid),
                preface: req.preface.clone(),
                ..PlaybookSpec::default()
            },
        );

        let playbook = playbook::create(&ctx.k8s, &resource).await.map_err(|err| {
            error!("{:?}", err);
            ApiError::KubernetesError
        })?;

        Ok(PlaybookResponse {
            id: playbook.name_any(),
            title: playbook.spec.title,
            description: playbook.spec.description,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub async fn update(
        _ctx: Arc<Context>,
        _id: Uuid,
        _title: Option<String>,
        _description: Option<String>,
    ) -> Result<PlaybookResponse> {
        unimplemented!()
    }
}
