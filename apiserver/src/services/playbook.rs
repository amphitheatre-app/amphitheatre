// Copyright (c) The Amphitheatre Authors. All rights reserved.
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

use amp_common::resource::{Playbook, PlaybookSpec};
use amp_resources::playbook;
use uuid::Uuid;

use crate::context::Context;
use crate::errors::ApiError;
use crate::requests::playbook::{CreatePlaybookRequest, UpdatePlaybookRequest};
use crate::services::Result;

pub struct PlaybookService;

impl PlaybookService {
    pub async fn get(ctx: Arc<Context>, id: Uuid) -> Result<PlaybookSpec> {
        let playbook = playbook::get(&ctx.k8s, &id.to_string()).await.map_err(ApiError::ResourceError)?;

        Ok(playbook.spec)
    }

    pub async fn list(ctx: Arc<Context>) -> Result<Vec<PlaybookSpec>> {
        let resources = playbook::list(&ctx.k8s).await.map_err(ApiError::ResourceError)?;

        Ok(resources.iter().map(|playbook| playbook.spec.clone()).collect())
    }

    pub async fn start(_ctx: Arc<Context>, _id: Uuid) -> Result<()> {
        unimplemented!()
    }

    pub async fn stop(_ctx: Arc<Context>, _id: Uuid) -> Result<()> {
        unimplemented!()
    }

    pub async fn delete(ctx: Arc<Context>, id: Uuid) -> Result<()> {
        playbook::delete(&ctx.k8s, &id.to_string()).await.map_err(ApiError::ResourceError)?;

        Ok(())
    }

    pub async fn create(ctx: Arc<Context>, req: &CreatePlaybookRequest) -> Result<PlaybookSpec> {
        let uuid = Uuid::new_v4();
        let resource = Playbook::new(
            &uuid.to_string(),
            PlaybookSpec {
                id: uuid.to_string(),
                title: req.title.to_string(),
                description: req.description.clone(),
                preface: req.preface.clone(),
                ..PlaybookSpec::default()
            },
        );

        let playbook = playbook::create(&ctx.k8s, &resource).await.map_err(ApiError::ResourceError)?;

        Ok(playbook.spec)
    }

    pub async fn update(_ctx: Arc<Context>, _id: Uuid, _req: &UpdatePlaybookRequest) -> Result<PlaybookSpec> {
        unimplemented!()
    }
}
