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

pub struct ExposingState;

#[async_trait]
impl State<Actor> for ExposingState {
    /// Execute the logic for the exposing state
    async fn handle(&self, ctx: &Context<Actor>) -> Option<Box<dyn State<Actor>>> {
        // Check if ExposeTask should be executed
        let task = ExposeTask::new();
        if task.matches(ctx) {
            if let Err(err) = task.execute(ctx).await {
                error!("Error during ExposeTask execution: {}", err);
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

    fn matches(&self, _: &Context<Actor>) -> bool {
        true
    }

    /// Execute the task logic for ExposeTask using shared data
    async fn execute(&self, _: &Context<Actor>) -> Result<()> {
        Ok(())
    }
}
