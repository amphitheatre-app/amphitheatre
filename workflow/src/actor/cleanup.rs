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
use crate::{Context, State, Task};

use amp_common::resource::Actor;

use async_trait::async_trait;
use k8s_openapi::api::core::v1::Namespace;
use kube::{Api, ResourceExt};
use tracing::info;

pub struct CleanupState;

#[async_trait]
impl State<Actor> for CleanupState {
    /// Execute the logic for the cleanup state
    async fn handle(&self, ctx: &Context<Actor>) -> Option<Box<dyn State<Actor>>> {
        // Check if CleanupTask should be executed
        let task = CleanupTask::new();
        if task.matches(ctx) {
            if let Err(err) = task.execute(ctx).await {
                // Handle error, maybe log it
                println!("Error during CleanupTask execution: {}", err);
            }
        }

        None // No transition, end of workflow
    }
}

pub struct CleanupTask;

#[async_trait]
impl Task<Actor> for CleanupTask {
    fn new() -> Self {
        CleanupTask
    }

    fn matches(&self, _: &Context<Actor>) -> bool {
        // Always true, this task is called directly from the controller
        true
    }

    // Execute the task logic for CleanupTask using shared data
    async fn execute(&self, ctx: &Context<Actor>) -> Result<()> {
        self.cleanup(ctx, &ctx.object).await
    }
}

impl CleanupTask {
    async fn cleanup(&self, ctx: &Context<Actor>, actor: &Actor) -> Result<()> {
        let namespace = actor.namespace().unwrap();
        let api: Api<Namespace> = Api::all((*ctx.k8s).clone());

        let ns = api.get(namespace.as_str()).await.map_err(Error::KubeError)?;
        if let Some(status) = ns.status {
            if status.phase == Some("Terminating".into()) {
                return Ok(());
            }
        }

        info!("Delete Actor `{}`", actor.name_any());

        Ok(())
    }
}
