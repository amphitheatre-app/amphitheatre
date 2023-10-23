// Copyright 2023 The Amphitheatre Authors.
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

// use crate::response::ApiError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("SerializationError: {0}")]
    SerializationError(#[source] serde_json::Error),

    #[error("Kube Error: {0}")]
    KubeError(#[source] kube::Error),

    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),

    #[error("UrlParseError: {0}")]
    UrlParseError(#[source] url::ParseError),
    // #[error("ApiError: {0}")]
    // ApiError(#[source] ApiError),
    #[error("Metrics not available at the moment")]
    MetricsNotAvailable,

    #[error("Unknown Syncer: {0}")]
    UnknownSyncer(String),

    #[error("Unknown Builder: {0}")]
    UnknownBuilder(String),

    #[error("DockerRegistryExistsFailed: {0}")]
    DockerRegistryExistsFailed(#[source] anyhow::Error),

    #[error("MissingSyncer")]
    MissingSyncer,

    #[error("MissingBuilder")]
    MissingBuilder,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
