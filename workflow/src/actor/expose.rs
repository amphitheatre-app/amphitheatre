// Copyright (c) The Amphitheatre Authors. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::errors::{Error, Result};
use crate::{Context, Intent, State, Task};

use amp_common::resource::Actor;

use amp_resources::service;
use async_trait::async_trait;
use kube::ResourceExt;
use tracing::{error, info, trace};

pub struct ExposingState;

#[async_trait]
impl State<Actor> for ExposingState {
    /// Execute the logic for the exposing state
    async fn handle(&self, ctx: &Context<Actor>) -> Option<Intent<Actor>> {
        trace!("Checking exposing state of actor {}", ctx.object.name_any());

        // Check if ExposeTask should be executed
        let task = ExposeTask::new();
        if task.matches(ctx) {
            match task.execute(ctx).await {
                Ok(Some(intent)) => return Some(intent),
                Err(err) => error!("Error during ExposeTask execution: {}", err),
                Ok(None) => {}
            }
        }

        None // No transition, wait for next state
    }
}

pub struct ExposeTask;

#[async_trait]
impl Task<Actor> for ExposeTask {
    fn new() -> Self {
        ExposeTask
    }

    fn matches(&self, ctx: &Context<Actor>) -> bool {
        ctx.object.status.as_ref().is_some_and(|status| status.running()) && ctx.object.spec.has_services()
    }

    /// Execute the task logic for ExposeTask using shared data
    async fn execute(&self, ctx: &Context<Actor>) -> Result<Option<Intent<Actor>>> {
        self.serve(ctx, &ctx.object).await.map_err(Error::ResourceError)?;
        Ok(None)
    }
}

impl ExposeTask {
    async fn serve(&self, ctx: &Context<Actor>, actor: &Actor) -> Result<(), amp_resources::error::Error> {
        let name = actor.name_any();
        match service::exists(&ctx.k8s, actor).await? {
            true => {
                info!("Try to refresh an existing Service {name}");
                service::update(&ctx.k8s, actor).await?;
            }
            false => {
                service::create(&ctx.k8s, actor).await?;
                info!("Created new Service: {name}");
            }
        }

        Ok(())
    }
}
