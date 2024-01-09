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

use amp_common::resource::Character;
use kube::{Api, Client};

use super::error::{Error, Result};

/// Get a character by name
pub async fn get(client: &Client, name: &str) -> Result<Character> {
    let api: Api<Character> = Api::all(client.clone());
    let resources = api.get(name).await.map_err(Error::KubeError)?;

    Ok(resources)
}
