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

use std::sync::Arc;

use crate::{errors::Error, Builder, Result};

use amp_common::{config::Credentials, resource::Actor};
use amp_resources::{
    kpack::{cluster_builder, cluster_buildpack, cluster_store, image, syncer, BuildExt},
    volume,
};

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::info;

/// Buildpacks builder implementation using Buildpacks kpack.
pub struct KpackBuilder {
    k8s: Arc<kube::Client>,
    credentials: Arc<RwLock<Credentials>>,
    actor: Arc<Actor>,
}

impl KpackBuilder {
    pub fn new(k8s: Arc<kube::Client>, actor: Arc<Actor>, credentials: Arc<RwLock<Credentials>>) -> Self {
        Self { k8s, credentials, actor }
    }
}

#[async_trait]
impl Builder for KpackBuilder {
    async fn build(&self) -> Result<()> {
        // initialize the some resources before building
        self.try_init_pvc().await.map_err(Error::ResourceError)?;
        self.try_init_syncer().await.map_err(Error::ResourceError)?;
        self.try_init_buildpack().await.map_err(Error::ResourceError)?;
        self.try_init_store().await.map_err(Error::ResourceError)?;
        self.try_init_builder().await.map_err(Error::ResourceError)?;

        // Build or update the build job
        let name = format!("{}-builder", &self.actor.spec.name);
        match image::exists(&self.k8s, &self.actor).await.map_err(Error::ResourceError)? {
            true => {
                // Build job already exists, update it if there are new changes
                info!("Try to refresh an existing build Job {}", name);
                image::update(&self.k8s, &self.actor).await.map_err(Error::ResourceError)?;
            }
            false => {
                info!("Create new build Job: {}", name);
                image::create(&self.k8s, &self.actor).await.map_err(Error::ResourceError)?;
            }
        }

        Ok(())
    }
}

impl KpackBuilder {
    async fn try_init_pvc(&self) -> Result<(), amp_resources::error::Error> {
        if !self.actor.spec.live {
            return Ok(());
        }

        if !volume::exists(&self.k8s, &self.actor).await? {
            volume::create(&self.k8s, &self.actor).await?;
        }

        Ok(())
    }

    async fn try_init_buildpack(&self) -> Result<(), amp_resources::error::Error> {
        let buildpacks = self.actor.spec.character.buildpacks();
        if buildpacks.is_none() {
            return Ok(());
        }

        for buildpack in buildpacks.unwrap() {
            if !cluster_buildpack::exists(&self.k8s, buildpack).await? {
                cluster_buildpack::create(&self.k8s, buildpack).await?;
            }
        }

        Ok(())
    }

    async fn try_init_store(&self) -> Result<(), amp_resources::error::Error> {
        if !cluster_store::exists(&self.k8s, &self.actor).await? {
            cluster_store::create(&self.k8s, &self.actor).await?;
        }

        Ok(())
    }
    async fn try_init_builder(&self) -> Result<(), amp_resources::error::Error> {
        if !cluster_builder::exists(&self.k8s, &self.actor).await? {
            cluster_builder::create(&self.k8s, &self.actor, self.credentials.clone()).await?;
        }

        Ok(())
    }

    async fn try_init_syncer(&self) -> Result<(), amp_resources::error::Error> {
        if !syncer::exists(&self.k8s, &self.actor).await? {
            syncer::create(&self.k8s, &self.actor).await?;
        }

        Ok(())
    }
}
