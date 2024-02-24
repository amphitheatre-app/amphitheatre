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

mod lifecycle;
use std::time::Duration;

pub use lifecycle::LifecycleBuilder;

mod kaniko;
pub use kaniko::KanikoBuilder;

mod kpack;
pub use kpack::KpackBuilder;

pub mod errors;
use errors::Result;

use async_trait::async_trait;

/// Builder trait
#[async_trait]
pub trait Builder: Send + Sync {
    async fn prepare(&self) -> Result<Option<Duration>>;
    async fn build(&self) -> Result<()>;
    async fn completed(&self) -> Result<bool>;
}

/// Build director, it's a strategy pattern implementation
pub struct BuildDirector {
    builder: Box<dyn Builder>,
}

impl BuildDirector {
    /// Constructor, receive a builder implementation
    pub fn new(builder: Box<dyn Builder>) -> Self {
        BuildDirector { builder }
    }

    /// Change the builder
    pub fn set_builder(&mut self, builder: Box<dyn Builder>) {
        self.builder = builder;
    }

    /// Prepare the build
    pub async fn prepare(&self) -> Result<Option<Duration>> {
        self.builder.prepare().await
    }

    /// Execute the build logic
    pub async fn build(&self) -> Result<()> {
        self.builder.build().await
    }

    /// Check if the build is completed
    pub async fn completed(&self) -> Result<bool> {
        self.builder.completed().await
    }
}

#[cfg(test)]
mod tests {
    use amp_common::{
        config::Credentials,
        resource::{Actor, ActorSpec},
    };

    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_build_director_lifecycle() {
        // only run this test in k8s environment
        let k8s = kube::Client::try_default().await;
        if let Ok(k8s) = k8s {
            let k8s = Arc::new(k8s);
            let actor = Arc::new(Actor::new("test", ActorSpec::default()));
            let builder = LifecycleBuilder::new(k8s, actor);
            let _ = BuildDirector::new(Box::new(builder));
        }
    }

    #[tokio::test]
    async fn test_build_director_kaniko() {
        // only run this test in k8s environment
        let k8s = kube::Client::try_default().await;
        if let Ok(k8s) = k8s {
            let k8s = Arc::new(k8s);
            let actor = Arc::new(Actor::new("test", ActorSpec::default()));
            let builder = KanikoBuilder::new(k8s, actor);
            let _ = BuildDirector::new(Box::new(builder));
        }
    }

    #[tokio::test]
    async fn test_build_director_kpack() {
        // only run this test in k8s environment
        let k8s = kube::Client::try_default().await;
        if let Ok(k8s) = k8s {
            let k8s = Arc::new(k8s);
            let actor = Arc::new(Actor::new("test", ActorSpec::default()));
            let builder = KpackBuilder::new(k8s, actor, Arc::new(RwLock::new(Credentials::default())));
            let _ = BuildDirector::new(Box::new(builder));
        }
    }
}
