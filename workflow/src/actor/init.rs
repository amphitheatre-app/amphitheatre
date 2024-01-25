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

use amp_common::resource::{Actor, ActorState};

use amp_resources::actor;
use async_trait::async_trait;
use kube::ResourceExt;
use tracing::{error, trace};

use super::BuildingState;

pub struct InitialState;

#[async_trait]
impl State<Actor> for InitialState {
    /// Execute the logic for the initial state
    async fn handle(&self, ctx: &Context<Actor>) -> Option<Intent<Actor>> {
        trace!("Checking initial state of actor {}", ctx.object.name_any());

        // Check if InitTask should be executed
        let task = InitTask::new();
        if task.matches(ctx) {
            match task.execute(ctx).await {
                Ok(Some(intent)) => return Some(intent),
                Err(err) => error!("Error during InitTask execution: {}", err),
                Ok(None) => {}
            }
        }

        // Transition to the next state if needed
        Some(Intent::State(Box::new(BuildingState)))
    }
}

pub struct InitTask;

#[async_trait]
impl Task<Actor> for InitTask {
    fn new() -> Self {
        InitTask
    }

    fn matches(&self, ctx: &Context<Actor>) -> bool {
        ctx.object.status.as_ref().is_some_and(|status| status.pending())
    }

    /// Execute the task logic for InitTask using shared data
    async fn execute(&self, ctx: &Context<Actor>) -> Result<Option<Intent<Actor>>> {
        let condition = ActorState::building();
        actor::patch_status(&ctx.k8s, &ctx.object, condition).await.map_err(Error::ResourceError)?;

        Ok(None)
    }
}
