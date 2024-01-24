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

use amp_common::resource::{Partner, Playbook, PlaybookState};
use amp_resolver::partner::load;

use amp_resources::playbook;
use async_trait::async_trait;
use std::collections::HashSet;
use tracing::{debug, info};

use super::PerformingState;

pub struct SchedulingState;

#[async_trait]
impl State<Playbook> for SchedulingState {
    /// Execute the logic for the scheduling state
    async fn handle(&self, ctx: &Context<Playbook>) -> Option<Box<dyn State<Playbook>>> {
        // Check if ScheduleTask should be executed
        let task = ScheduleTask::new();
        if task.matches(ctx) {
            if let Err(err) = task.execute(ctx).await {
                // Handle error, maybe log it
                println!("Error during ScheduleTask execution: {}", err);
            }
        }

        // Transition to the next state if needed
        Some(Box::new(PerformingState))
    }
}

pub struct ScheduleTask;

#[async_trait]
impl Task<Playbook> for ScheduleTask {
    fn new() -> Self {
        ScheduleTask
    }

    fn matches(&self, ctx: &Context<Playbook>) -> bool {
        ctx.object.status.as_ref().is_some_and(|status| status.resolving())
    }

    // Execute the task logic for InitTask using shared data
    async fn execute(&self, ctx: &Context<Playbook>) -> Result<()> {
        self.resolve(ctx, &ctx.object).await
    }
}

impl ScheduleTask {
    async fn resolve(&self, ctx: &Context<Playbook>, playbook: &Playbook) -> Result<()> {
        // Check if there are any repositories to fetch
        //
        let mut fetches: HashSet<(&str, Partner)> = HashSet::new();

        if let Some(characters) = &playbook.spec.characters {
            let exists: HashSet<&String> = characters.iter().map(|char| &char.meta.name).collect();
            debug!("The currently existing actors are: {exists:?}");

            for character in characters {
                if let Some(partners) = &character.partners {
                    for (name, partner) in partners {
                        if !exists.contains(name) {
                            fetches.insert((name, partner.clone()));
                        }
                    }
                }
            }

            debug!("The repositories to be fetched are: {fetches:?}");
        }

        // Fetch the actors from the repositories
        //
        let credentials = ctx.credentials.read().await;
        for (name, partner) in fetches.iter() {
            let character = load(&ctx.k8s, &credentials, name, partner).await.map_err(Error::ResolveError)?;
            playbook::add(&ctx.k8s, playbook, character).await.map_err(Error::ResourceError)?;
            info!("Fetch and add the actor to this playbook");
        }

        // If there are no repositories to fetch, then the resolution is complete.
        if fetches.is_empty() {
            let condition = PlaybookState::running(true, "AutoRun", None);
            playbook::patch_status(&ctx.k8s, playbook, condition).await.map_err(Error::ResourceError)?;
            info!("Resolved successfully, Running");
        }

        Ok(())
    }
}
