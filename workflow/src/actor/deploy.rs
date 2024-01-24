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

use crate::errors::Error;
use crate::errors::Result;
use crate::{Context, State, Task};

use amp_common::resource::Actor;

use amp_resources::deployer::Deployer;
use async_trait::async_trait;
use kube::ResourceExt;
use tracing::error;
use tracing::info;

use super::ExposingState;

pub struct DeployingState;

#[async_trait]
impl State<Actor> for DeployingState {
    /// Execute the logic for the deploying state
    async fn handle(&self, ctx: &Context<Actor>) -> Option<Box<dyn State<Actor>>> {
        // Check if DeployTask should be executed
        let task = DeployTask::new();
        if task.matches(ctx) {
            if let Err(err) = task.execute(ctx).await {
                error!("Error during DeployTask execution: {}", err);
            }
        }

        // Transition to the next state if needed
        Some(Box::new(ExposingState))
    }
}

pub struct DeployTask;

#[async_trait]
impl Task<Actor> for DeployTask {
    fn new() -> Self {
        DeployTask
    }

    fn matches(&self, _: &Context<Actor>) -> bool {
        true
    }

    /// Execute the task logic for DeployTask using shared data
    async fn execute(&self, ctx: &Context<Actor>) -> Result<()> {
        info!("Try to deploying the resources for Actor {}", &ctx.object.name_any());

        let credentials = ctx.credentials.read().await;
        let mut deployer = Deployer::new((*ctx.k8s).clone(), &credentials, &ctx.object);
        deployer.run().await.map_err(Error::DeployError)?;

        Ok(())
    }
}
