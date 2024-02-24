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

use std::{sync::Arc, time::Duration};

use crate::{errors::Error, Builder, Result};

use amp_common::resource::Actor;
use amp_resources::{containers::kaniko, job};

use async_trait::async_trait;
use tracing::info;

/// Dockerfile builder implementation using Kaniko.
pub struct KanikoBuilder {
    k8s: Arc<kube::Client>,
    actor: Arc<Actor>,
}

impl KanikoBuilder {
    pub fn new(k8s: Arc<kube::Client>, actor: Arc<Actor>) -> Self {
        Self { k8s, actor }
    }
}

#[async_trait]
impl Builder for KanikoBuilder {
    // initialize the some resources before building
    async fn prepare(&self) -> Result<Option<Duration>> {
        Ok(None) // No need to wait
    }

    async fn build(&self) -> Result<()> {
        let name = format!("{}-builder", &self.actor.spec.name);
        let pod = kaniko::pod(&self.actor).map_err(Error::ResourceError)?;

        // Build or update the build job
        match job::exists(&self.k8s, &self.actor).await.map_err(Error::ResourceError)? {
            true => {
                // Build job already exists, update it if there are new changes
                info!("Try to refresh an existing build Job {}", name);
                job::update(&self.k8s, &self.actor, pod).await.map_err(Error::ResourceError)?;
            }
            false => {
                info!("Create new build Job: {}", name);
                job::create(&self.k8s, &self.actor, pod).await.map_err(Error::ResourceError)?;
            }
        }

        Ok(())
    }

    #[inline]
    async fn completed(&self) -> Result<bool> {
        job::completed(&self.k8s, &self.actor).await.map_err(Error::ResourceError)
    }
}
