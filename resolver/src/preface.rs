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

use crate::{
    errors::{ResolveError, Result},
    load_from_catalog, load_from_cluster, load_from_source,
};
use amp_common::{config::Credentials, resource::CharacterSpec, resource::Preface};
use kube::Client as KubeClient;

/// Load manifest from different sources and return the actor spec.
pub async fn load(client: &KubeClient, credentials: &Credentials, preface: &Preface) -> Result<CharacterSpec> {
    if let Some(p) = &preface.registry {
        let registry = p.registry.clone().unwrap_or_else(|| "catalog".to_string());
        return match registry.as_str() {
            "catalog" => load_from_catalog(credentials, &preface.name, &p.version),
            "hub" => load_from_cluster(client, &preface.name).await,
            x => Err(ResolveError::UnknownCharacterRegistry(x.to_string())),
        };
    }

    if let Some(reference) = &preface.repository {
        return load_from_source(credentials, reference);
    }

    if let Some(manifest) = &preface.manifest {
        return Ok(manifest.clone());
    }

    Err(ResolveError::UnknownPreface)
}
