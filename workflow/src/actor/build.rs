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

use std::time::Duration;

use super::DeployingState;
use crate::errors::{Error, Result};
use crate::{Context, Intent, State, Task};

use amp_builder::{BuildDirector, KanikoBuilder, KpackBuilder};
use amp_common::resource::{Actor, ActorState};
use amp_common::schema::BuildMethod;

use amp_resources::actor;
use async_trait::async_trait;
use kube::runtime::controller::Action;
use kube::ResourceExt;
use tracing::{error, info, trace};

pub struct BuildingState;

#[async_trait]
impl State<Actor> for BuildingState {
    /// Execute the logic for the building state
    async fn handle(&self, ctx: &Context<Actor>) -> Option<Intent<Actor>> {
        trace!("Checking building state of actor {}", ctx.object.name_any());

        // Check if BuildTask should be executed
        let task = BuildTask::new();
        if task.matches(ctx) {
            match task.execute(ctx).await {
                Ok(Some(intent)) => return Some(intent),
                Err(err) => error!("Error during BuildTask execution: {}", err),
                Ok(None) => {}
            }
        }

        // Transition to the next state if needed
        Some(Intent::State(Box::new(DeployingState)))
    }
}

pub struct BuildTask;

#[async_trait]
impl Task<Actor> for BuildTask {
    fn new() -> Self {
        BuildTask
    }

    fn matches(&self, ctx: &Context<Actor>) -> bool {
        ctx.object.status.as_ref().is_some_and(|status| status.building())
    }

    /// Execute the task logic for BuildTask using shared data.
    async fn execute(&self, ctx: &Context<Actor>) -> Result<Option<Intent<Actor>>> {
        let actor = &ctx.object;
        let build = actor.spec.character.build.clone().unwrap_or_default();

        // Generate `Builder` based on the build method
        let builder = match build.method() {
            BuildMethod::Dockerfile => {
                info!("Found dockerfile, build it with Kaniko");
                BuildDirector::new(Box::new(KanikoBuilder::new(ctx.k8s.clone(), actor.clone())))
            }
            BuildMethod::Buildpacks => {
                info!("Build the image with Cloud Native Buildpacks (kpack)");
                BuildDirector::new(Box::new(KpackBuilder::new(ctx.k8s.clone(), actor.clone(), ctx.credentials.clone())))
            }
        };

        // Build the image
        builder.build().await.map_err(Error::BuildError)?;

        // Check if the build is completed and wait for it to finish.
        if !builder.completed().await.map_err(Error::BuildError)? {
            info!("Build job is not completed yet, wait for it to finish");
            return Ok(Some(Intent::Action(Action::requeue(Duration::from_secs(5)))));
        }

        // Patch the status to running
        let condition = ActorState::running(true, "AutoRun", None);
        actor::patch_status(&ctx.k8s, &ctx.object, condition).await.map_err(Error::ResourceError)?;

        Ok(None)
    }
}
