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

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Serialization Error: {0}")]
    ResourceError(#[source] amp_resources::error::Error),

    #[error("Kube Error: {0}")]
    KubeError(#[source] kube::Error),

    #[error("Resolve Error: {0}")]
    ResolveError(#[source] amp_resolver::errors::ResolveError),

    #[error("Nats Error: {0}")]
    NatsError(#[from] async_nats::Error),

    #[error("Deploy Error: {0}")]
    DeployError(#[source] amp_resources::error::Error),

    #[error(" Docker Credential Error for {0}: {1}")]
    DockerCredentialError(String, amp_common::docker::errors::CredentialError),

    #[error("Docker Registry Error: {0}")]
    DockerRegistryError(#[source] anyhow::Error),

    #[error("Build Error: {0}")]
    BuildError(#[source] amp_builder::errors::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
