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

use amp_common::resource::Actor;
use amp_resources::containers::application;
use amp_resources::deployment;
use amp_resources::error::Error as ResourceError;
use amp_resources::hash;

use async_trait::async_trait;
use k8s_openapi::api::core::v1::PodSpec;
use kube::ResourceExt;
use tracing::trace;
use tracing::{error, info};

use super::ExposingState;

pub struct DeployingState;

#[async_trait]
impl State<Actor> for DeployingState {
    /// Execute the logic for the deploying state
    async fn handle(&self, ctx: &Context<Actor>) -> Option<Intent<Actor>> {
        trace!("Checking deploying state of actor {}", ctx.object.name_any());

        // Check if DeployTask should be executed
        let task = DeployTask::new();
        if task.matches(ctx) {
            match task.execute(ctx).await {
                Ok(Some(intent)) => return Some(intent),
                Err(err) => error!("Error during DeployTask execution: {}", err),
                Ok(None) => {}
            }
        }

        // Transition to the next state
        Some(Intent::State(Box::new(ExposingState)))
    }
}

pub struct DeployTask;

#[async_trait]
impl Task<Actor> for DeployTask {
    fn new() -> Self {
        DeployTask
    }

    fn matches(&self, ctx: &Context<Actor>) -> bool {
        ctx.object.status.as_ref().is_some_and(|status| status.running())
    }

    /// Execute the task logic for DeployTask using shared data
    async fn execute(&self, ctx: &Context<Actor>) -> Result<Option<Intent<Actor>>> {
        info!("Try to deploying the resources for Actor {}", &ctx.object.name_any());
        self.deploy(ctx, &ctx.object).await.map_err(Error::DeployError)?;

        Ok(None)
    }
}

impl DeployTask {
    async fn deploy(&self, ctx: &Context<Actor>, actor: &Actor) -> Result<(), ResourceError> {
        let name = actor.name_any();
        let namespace = actor.namespace().ok_or_else(|| ResourceError::MissingObjectKey(".metadata.namespace"))?;

        let resource = deployment::new(actor, self.pod(actor))?;
        match deployment::exists(&ctx.k8s, &namespace, &name).await? {
            true => {
                // Deployment already exists, update it if there are new changes
                info!("Try to refresh an existing Deployment {name}");
                let expected_hash = hash(&actor.spec)?;
                deployment::update(&ctx.k8s, &namespace, &name, resource, expected_hash).await?;
            }
            false => {
                // Create a new Deployment
                deployment::create(&ctx.k8s, &namespace, resource).await?;
                info!("Created new Deployment: {name}");
            }
        }

        Ok(())
    }

    fn pod(&self, actor: &Actor) -> PodSpec {
        PodSpec { containers: vec![application::container(&actor.spec)], ..Default::default() }
    }
}
