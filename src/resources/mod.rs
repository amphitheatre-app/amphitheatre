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

use std::collections::HashMap;

use k8s_openapi::api::core::v1::EnvVar;
use serde::Serialize;
use serde_json::to_string;
use sha2::{Digest, Sha256};

use self::error::{Error, Result};

pub mod actor;
pub mod crds;
pub mod deployment;
pub mod error;
pub mod event;
pub mod image;
pub mod job;
pub mod namespace;
pub mod playbook;
pub mod secret;
pub mod service;
pub mod service_account;

const LAST_APPLIED_HASH_KEY: &str = "actors.amphitheatre.app/last-applied-hash";
const DEFAULT_GITSYNC_IMAGE: &str = "registry.k8s.io/git-sync/git-sync:v3.6.4";
const DEFAULT_KANIKO_IMAGE: &str = "gcr.io/kaniko-project/executor:latest";
const DEFAULT_PACK_IMAGE: &str = "buildpacksio/pack:latest";
const DEFAULT_BP_BUILDER: &str = "gcr.io/buildpacks/builder:v1";

pub fn hash<T>(resource: &T) -> Result<String>
where
    T: Serialize,
{
    let data = to_string(resource).map_err(Error::SerializationError)?;
    let hash = Sha256::digest(data);

    Ok(format!("{:x}", hash))
}

pub fn to_env_var<T: ToString>(env: &HashMap<T, T>) -> Vec<EnvVar> {
    env.iter()
        .map(|(key, value)| EnvVar {
            name: key.to_string(),
            value: Some(value.to_string()),
            value_from: None,
        })
        .collect()
}
