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

use amp_common::config::CredentialConfiguration;
use amp_common::schema::{ActorSpec, EitherCharacter, GitReference, Manifest};
use amp_common::scm::client::Client;
use errors::{ResolveError, Result};
use tracing::debug;

pub mod errors;
pub mod patches;
pub mod utils;

/// Load mainfest from different sources and return the actor spec.
pub fn load(configuration: &CredentialConfiguration, character: &EitherCharacter) -> Result<ActorSpec> {
    match character {
        EitherCharacter::Manifest(content) => load_from_manifest(configuration, content.as_bytes()),
        EitherCharacter::Name(name) => load_from_cluster(name),
        EitherCharacter::Git(reference) => load_from_source(configuration, reference),
    }
}

/// Read Character manifest from original manifest content and return the actor spec.
pub fn load_from_manifest(configuration: &CredentialConfiguration, content: &[u8]) -> Result<ActorSpec> {
    let manifest: Manifest = toml::from_slice(content).map_err(|e| ResolveError::TomlParseFailed(e.to_string()))?;
    let client = Client::init(configuration, &manifest.repository).map_err(ResolveError::SCMError)?;

    let mut spec = ActorSpec::from(&manifest);
    spec.source = patches::source(&client, &spec.source)?;
    spec.image = patches::image(configuration, &spec)?;

    Ok(spec)
}

/// @TODO: Load mainfest from Kubernetes cluster and return the actor spec.
pub fn load_from_cluster(_name: &str) -> Result<ActorSpec> {
    todo!()
}

/// Load mainfest from remote VCS (like github) and return the actor spec.
pub fn load_from_source(configuration: &CredentialConfiguration, reference: &GitReference) -> Result<ActorSpec> {
    let client = Client::init(configuration, &reference.repo).map_err(ResolveError::SCMError)?;

    let source = patches::source(&client, reference)?;
    let path = source.path.clone().unwrap_or(".amp.toml".into());
    let repo = utils::repo(&source.repo)?;

    let content = client
        .contents()
        .find(&repo, &path, &reference.rev())
        .map_err(|e| ResolveError::FetchingError(e.to_string()))?;
    debug!("The `.amp.toml` content of {} is:\n{:?}", repo, content);

    load_from_manifest(configuration, content.data.as_slice())
}
