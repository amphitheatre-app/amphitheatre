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

use amp_common::{config::Credentials, resource::Actor};
use amp_resources::{
    kpack::{
        cluster_builder, cluster_buildpack, cluster_store, encode_name, image, syncer,
        types::{find_top_level_buildpacks, Buildpack, Group, Order},
        BuildExt,
    },
    volume,
};

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, info};

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
    // initialize the some resources before building
    async fn prepare(&self) -> Result<Option<Duration>> {
        self.try_init_pvc().await.map_err(Error::ResourceError)?;

        // Check if the syncer is ready
        if let Some(duration) = self.try_init_syncer().await.map_err(Error::ResourceError)? {
            return Ok(Some(duration));
        }

        // Check if the buildpacks are ready
        if let Some(duration) = self.try_init_buildpack().await.map_err(Error::ResourceError)? {
            return Ok(Some(duration));
        }

        // Check if the builder is ready
        if let Some(duration) = self.try_init_builder().await.map_err(Error::ResourceError)? {
            return Ok(Some(duration));
        }

        Ok(None)
    }

    async fn build(&self) -> Result<()> {
        // Build or update the Image
        let name = format!("{}-builder", &self.actor.spec.name);
        match image::exists(&self.k8s, &self.actor).await.map_err(Error::ResourceError)? {
            true => {
                // Image already exists, update it if there are new changes
                info!("Try to refresh an existing Image {}", name);
                image::update(&self.k8s, &self.actor).await.map_err(Error::ResourceError)?;
            }
            false => {
                info!("Create new Image: {}", name);
                image::create(&self.k8s, &self.actor).await.map_err(Error::ResourceError)?;
            }
        }

        Ok(())
    }

    #[inline]
    async fn completed(&self) -> Result<bool> {
        image::completed(&self.k8s, &self.actor).await.map_err(Error::ResourceError)
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

    async fn try_init_buildpack(&self) -> Result<Option<Duration>, amp_resources::error::Error> {
        let buildpacks = self.actor.spec.character.buildpacks();
        if buildpacks.is_none() {
            return Ok(None);
        }

        for buildpack in buildpacks.unwrap() {
            if !cluster_buildpack::exists(&self.k8s, buildpack).await? {
                cluster_buildpack::create(&self.k8s, buildpack).await?;
            }

            if !cluster_buildpack::ready(&self.k8s, buildpack).await? {
                return Ok(Some(Duration::from_secs(5))); // wait for the ClusterBuildpack to be ready
            }
        }

        Ok(None)
    }

    async fn try_init_builder(&self) -> Result<Option<Duration>, amp_resources::error::Error> {
        if !cluster_builder::exists(&self.k8s, &self.actor).await? {
            if !cluster_store::exists(&self.k8s, &self.actor).await? {
                cluster_store::create(&self.k8s, &self.actor).await?;
            }

            if !cluster_store::ready(&self.k8s, &self.actor).await? {
                return Ok(Some(Duration::from_secs(5))); // wait for the ClusterStore to be ready
            }

            let mut order = Vec::new();
            let store = cluster_store::get(&self.k8s, &self.actor).await?;
            if let Some(buildpacks) = store.data.pointer("/status/buildpacks") {
                let buildpacks: Vec<Buildpack> = serde_json::from_value(buildpacks.clone())
                    .map_err(amp_resources::error::Error::SerializationError)?;
                order = find_top_level_buildpacks(&buildpacks);
            }

            if let Some(buildpacks) = self.actor.spec.character.buildpacks() {
                order.append(
                    &mut buildpacks
                        .iter()
                        .map(|item| Order {
                            group: vec![Group {
                                name: Some(encode_name(item)),
                                kind: Some("ClusterBuildpack".to_string()),
                                ..Default::default()
                            }],
                        })
                        .collect(),
                );
            }

            debug!("The ClusterBuilder order: {:?}", order);

            let credentials = self.credentials.read().await;
            let tag = self.actor.spec.character.builder_tag(&credentials)?;
            cluster_builder::create(&self.k8s, &self.actor, &tag, order).await?;
        }

        if !cluster_builder::ready(&self.k8s, &self.actor).await? {
            return Ok(Some(Duration::from_secs(5))); // wait for the ClusterBuilder to be ready
        }

        Ok(None)
    }

    async fn try_init_syncer(&self) -> Result<Option<Duration>, amp_resources::error::Error> {
        if !self.actor.spec.live {
            return Ok(None);
        }

        if !syncer::exists(&self.k8s, &self.actor).await? {
            syncer::create(&self.k8s, &self.actor).await?;
        }

        if !syncer::ready(&self.k8s, &self.actor).await? {
            return Ok(Some(Duration::from_secs(5))); // wait for the Syncer to be ready
        }

        Ok(None)
    }
}
