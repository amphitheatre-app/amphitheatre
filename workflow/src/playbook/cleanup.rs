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

use crate::errors::Result;
use crate::{Context, Intent, State, Task};
use amp_common::resource::Playbook;
use async_trait::async_trait;
use kube::ResourceExt;
use tracing::{error, info, trace};

pub struct CleanupState;

#[async_trait]
impl State<Playbook> for CleanupState {
    /// Execute the logic for the cleanup state
    async fn handle(&self, ctx: &Context<Playbook>) -> Option<Intent<Playbook>> {
        trace!("Checking cleanup state of playbook {}", ctx.object.name_any());

        // Check if EndTask should be executed
        let task = CleanupTask::new();
        if task.matches(ctx) {
            match task.execute(ctx).await {
                Ok(Some(intent)) => return Some(intent),
                Err(err) => error!("Error during CleanupTask execution: {}", err),
                Ok(None) => {}
            }
        }

        None // No transition, end of workflow
    }
}

pub struct CleanupTask;

#[async_trait]
impl Task<Playbook> for CleanupTask {
    fn new() -> Self {
        CleanupTask
    }

    fn matches(&self, _: &Context<Playbook>) -> bool {
        // Always true, this task is called directly from the controller
        true
    }

    // Execute the task logic for EndTask using shared data
    async fn execute(&self, ctx: &Context<Playbook>) -> Result<Option<Intent<Playbook>>> {
        self.cleanup(ctx, &ctx.object).await?;
        Ok(None)
    }
}

impl CleanupTask {
    async fn cleanup(&self, ctx: &Context<Playbook>, playbook: &Playbook) -> Result<()> {
        // Try to delete the NATS stream for this playbook if it exists.
        if ctx.jetstream.delete_stream(playbook.name_any()).await.is_ok() {
            info!("Deleted NATS stream for playbook {}", playbook.name_any());
        }

        Ok(())
    }
}
