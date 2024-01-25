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
use crate::Intent;
use crate::{Context, State, Task};

use amp_common::resource::{Playbook, PlaybookState};
use amp_resolver::preface::load;
use amp_resources::{namespace, playbook};

use async_trait::async_trait;
use kube::ResourceExt;
use tracing::{debug, error, info, trace};

use super::ResolvingState;

pub struct InitialState;

#[async_trait]
impl State<Playbook> for InitialState {
    /// Execute the logic for the initial state
    async fn handle(&self, ctx: &Context<Playbook>) -> Option<Intent<Playbook>> {
        trace!("Checking initial state of playbook {}", ctx.object.name_any());

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
        Some(Intent::State(Box::new(ResolvingState)))
    }
}

pub struct InitTask;

#[async_trait]
impl Task<Playbook> for InitTask {
    fn new() -> Self {
        InitTask
    }

    fn matches(&self, ctx: &Context<Playbook>) -> bool {
        ctx.object.status.as_ref().is_some_and(|status| status.pending())
    }

    /// Execute the task logic for InitTask using shared data
    async fn execute(&self, ctx: &Context<Playbook>) -> Result<Option<Intent<Playbook>>> {
        // Create namespace for this playbook
        namespace::create(&ctx.k8s, &ctx.object).await.map_err(Error::ResourceError)?;
        info!("Created namespace for playbook {}", ctx.object.name_any());

        // Add the preface to the playbook for first resolving
        self.add_preface(ctx, &ctx.object).await?;

        // Update the playbook status to resolving
        let condition = PlaybookState::resolving();
        playbook::patch_status(&ctx.k8s, &ctx.object, condition).await.map_err(Error::ResourceError)?;
        info!("Init successfully, Let's begin resolving, now!");

        Ok(None)
    }
}

impl InitTask {
    async fn add_preface(&self, ctx: &Context<Playbook>, playbook: &Playbook) -> Result<()> {
        debug!("Build from the starting characters (preface)");

        let preface = &playbook.spec.preface;
        let credentials = ctx.credentials.read().await;
        let character = load(&ctx.k8s, &credentials, preface).await.map_err(Error::ResolveError)?;
        playbook::add(&ctx.k8s, playbook, character).await.map_err(Error::ResourceError)?;
        info!("Fetch and add the character to this playbook");

        Ok(())
    }
}
