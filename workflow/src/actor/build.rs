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

use amp_builder::{BuildDirector, KanikoBuilder, LifecycleBuilder};
use amp_common::docker::{self, registry, DockerConfig};
use amp_common::resource::{Actor, ActorState};
use amp_common::schema::BuildMethod;

use amp_resources::{actor, job};
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

    /// Execute the task logic for BuildTask using shared data
    async fn execute(&self, ctx: &Context<Actor>) -> Result<Option<Intent<Actor>>> {
        // build if actor is live or the image is not built, else skip to next state
        if ctx.object.spec.live || !self.built(ctx).await? {
            self.build(ctx).await?;

            if !job::completed(&ctx.k8s, &ctx.object).await.map_err(Error::ResourceError)? {
                info!("Build job is not completed yet, wait for it to finish");
                return Ok(Some(Intent::Action(Action::requeue(Duration::from_secs(5)))));
            }
        }

        // patch the status to running
        let condition = ActorState::running(true, "AutoRun", None);
        actor::patch_status(&ctx.k8s, &ctx.object, condition).await.map_err(Error::ResourceError)?;

        Ok(None)
    }
}

impl BuildTask {
    /// Check if the image is already built
    async fn built(&self, ctx: &Context<Actor>) -> Result<bool> {
        let image = &ctx.object.spec.image;

        let credentials = ctx.credentials.read().await;
        let config = DockerConfig::from(&credentials.registries);

        let credential = match docker::get_credential(&config, image) {
            Ok(credential) => Some(credential),
            Err(err) => {
                error!("Error handling docker configuration: {}", err);
                None
            }
        };

        if registry::exists(image, credential).await.map_err(Error::DockerRegistryError)? {
            info!("The images already exists");
            return Ok(true);
        }

        Ok(false)
    }

    /// Generate `Builder` based on the build strategy
    ///
    /// The source code can be local or remote,
    /// and the build method can be dockerfile or buildpacks,
    /// and build frequency can be once or live.
    ///
    /// The build strategy is a matrix of 2x2x2 like below:
    /// - Case 1: remote source code (git-sync), dockerfile (kaniko), (sync once)
    /// - Case 2: remote source code (git-sync), dockerfile (kaniko), live (sync & keep watching the changes, unsupported)
    /// - Case 3: remote source code (git-sync), buildpacks (kpack), (sync once)
    /// - Case 4: remote source code (git-sync), buildpacks (kpack), live (sync & keep watching the changes, unsupported)
    /// - Case 5: local source code (amp-syncer), dockerfile (kaniko), (sync once)
    /// - Case 6: local source code (amp-syncer), dockerfile (kaniko), live (sync & keep watching the changes)
    /// - Case 7: local source code (amp-syncer), buildpacks (kpack), (sync once)
    /// - Case 8: local source code (amp-syncer), buildpacks (kpack), live (sync & keep watching the changes)
    ///
    async fn build(&self, ctx: &Context<Actor>) -> Result<()> {
        let actor = &ctx.object;
        let build = actor.spec.character.build.clone().unwrap_or_default();
        let builder = match build.method() {
            BuildMethod::Dockerfile => {
                info!("Found dockerfile, build it with Kaniko");
                BuildDirector::new(Box::new(KanikoBuilder::new(ctx.k8s.clone(), actor.clone())))

                // self.builder = Some(kaniko::container(&self.actor.spec));
            }
            BuildMethod::Buildpacks => {
                info!("Build the image with Cloud Native Buildpacks (Lifecycle)");
                BuildDirector::new(Box::new(LifecycleBuilder::new(ctx.k8s.clone(), actor.clone())))
            }
        };

        builder.build().await.map_err(Error::BuildError)
    }
}
