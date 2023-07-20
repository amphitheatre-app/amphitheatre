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

use serde::Serialize;
use serde_json::to_string;
use sha2::{Digest, Sha256};

use self::error::{Error, Result};

pub mod actor;
pub mod builder;
pub mod character;
pub mod credential;
pub mod deployment;
pub mod error;
pub mod event;
pub mod namespace;
pub mod playbook;
pub mod secret;
pub mod service;
pub mod service_account;

const LAST_APPLIED_HASH_KEY: &str = "amphitheatre.app/last-applied-hash";

pub fn hash<T>(resource: &T) -> Result<String>
where
    T: Serialize,
{
    let data = to_string(resource).map_err(Error::SerializationError)?;
    let hash = Sha256::digest(data);

    Ok(format!("{:x}", hash))
}

/// Returns a list of arguments in one-dash or two-dash style.
#[inline]
pub fn args(args: &[(&str, &str)], dash: i8) -> Vec<String> {
    args.iter()
        .map(|(key, value)| {
            if dash == 1 {
                format!("-{}={}", key, value)
            } else {
                format!("--{}={}", key, value)
            }
        })
        .collect()
}
