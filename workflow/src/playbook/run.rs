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
use amp_common::resource::Playbook;
use amp_resolver::to_actor;
use amp_resources::actor;
use async_trait::async_trait;
use kube::ResourceExt;
use tracing::{error, info, trace};

pub struct RunningState;

#[async_trait]
impl State<Playbook> for RunningState {
    /// Execute the logic for the running state
    async fn handle(&self, ctx: &Context<Playbook>) -> Option<Intent<Playbook>> {
        trace!("Checking running state of playbook {}", ctx.object.name_any());

        // Check if RunTask should be executed
        let task = RunTask::new();
        if task.matches(ctx) {
            match task.execute(ctx).await {
                Ok(Some(intent)) => return Some(intent),
                Err(err) => error!("Error during RunTask execution: {}", err),
                Ok(None) => {}
            }
        }

        None // No transition, wait for next state
    }
}

pub struct RunTask;

#[async_trait]
impl Task<Playbook> for RunTask {
    fn new() -> Self {
        RunTask
    }

    fn matches(&self, ctx: &Context<Playbook>) -> bool {
        ctx.object.status.as_ref().is_some_and(|status| status.running())
    }

    async fn execute(&self, ctx: &Context<Playbook>) -> Result<Option<Intent<Playbook>>> {
        self.run(ctx, &ctx.object).await?;
        Ok(None)
    }
}

impl RunTask {
    async fn run(&self, ctx: &Context<Playbook>, playbook: &Playbook) -> Result<()> {
        let credentials = ctx.credentials.read().await;

        if playbook.spec.characters.is_none() {
            error!("No characters defined in the playbook");
            return Ok(());
        }

        let characters = playbook.spec.characters.as_ref().unwrap();
        for character in characters {
            let name = &character.meta.name;
            match actor::exists(&ctx.k8s, playbook, name).await.map_err(Error::ResourceError)? {
                true => {
                    // Actor already exists, update it if there are new changes
                    info!("Try to refresh an existing Actor {}", name);

                    let spec = to_actor(character, &credentials).map_err(Error::ResolveError)?;
                    actor::update(&ctx.k8s, playbook, &spec).await.map_err(Error::ResourceError)?;
                }
                false => {
                    // Create a new actor
                    info!("Create new Actor: {}", name);

                    let spec = to_actor(character, &credentials).map_err(Error::ResolveError)?;
                    actor::create(&ctx.k8s, playbook, &spec).await.map_err(Error::ResourceError)?;
                }
            }
        }
        Ok(())
    }
}
