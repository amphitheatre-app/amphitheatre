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
use crate::{Context, State, Task};

use amp_common::resource::Actor;

use async_trait::async_trait;
use tracing::error;

use super::BuildingState;

pub struct InitialState;

#[async_trait]
impl State<Actor> for InitialState {
    /// Execute the logic for the initial state
    async fn handle(&self, ctx: &Context<Actor>) -> Option<Box<dyn State<Actor>>> {
        // Check if InitTask should be executed
        let task = InitTask::new();
        if task.matches(ctx) {
            if let Err(err) = task.execute(ctx).await {
                error!("Error during InitTask execution: {}", err);
            }
        }

        // Transition to the next state if needed
        Some(Box::new(BuildingState))
    }
}

pub struct InitTask;

#[async_trait]
impl Task<Actor> for InitTask {
    fn new() -> Self {
        InitTask
    }

    fn matches(&self, _: &Context<Actor>) -> bool {
        true
    }

    /// Execute the task logic for InitTask using shared data
    async fn execute(&self, _: &Context<Actor>) -> Result<()> {
        Ok(())
    }
}
