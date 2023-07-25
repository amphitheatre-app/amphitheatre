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

use amp_common::config::Credentials;
use amp_common::schema::{ActorSpec, EitherCharacter, GitReference, Manifest};
use amp_common::scm::client::Client as ScmClient;
use amp_resources::character;
use errors::{ResolveError, Result};
use kube::Client as KubeClient;
use tracing::debug;

pub mod errors;
pub mod patches;
pub mod utils;

/// Load mainfest from different sources and return the actor spec.
pub async fn load(client: &KubeClient, credentials: &Credentials, character: &EitherCharacter) -> Result<ActorSpec> {
    match character {
        EitherCharacter::Manifest(content) => {
            let manifest: Manifest = toml::from_str(content).map_err(ResolveError::TomlParseFailed)?;
            load_from_manifest(credentials, manifest)
        }
        EitherCharacter::Name(name) => load_from_cluster(client, credentials, name).await,
        EitherCharacter::Git(reference) => load_from_source(credentials, reference),
    }
}

/// Read Character manifest from original manifest content and return the actor spec.
pub fn load_from_manifest(credentials: &Credentials, manifest: Manifest) -> Result<ActorSpec> {
    let client = ScmClient::init(credentials, &manifest.repository).map_err(ResolveError::SCMError)?;

    let mut spec = ActorSpec::from(&manifest);
    spec.source = patches::source(&client, &spec.source)?;
    spec.image = patches::image(credentials, &spec)?;

    Ok(spec)
}

/// Load mainfest from Kubernetes cluster and return the actor spec.
pub async fn load_from_cluster(client: &KubeClient, credentials: &Credentials, name: &str) -> Result<ActorSpec> {
    let character = character::get(client, name)
        .await
        .map_err(ResolveError::ResourceError)?;
    load_from_manifest(credentials, character.spec)
}

/// Load mainfest from remote VCS (like github) and return the actor spec.
pub fn load_from_source(credentials: &Credentials, reference: &GitReference) -> Result<ActorSpec> {
    let client = ScmClient::init(credentials, &reference.repo).map_err(ResolveError::SCMError)?;

    let source = patches::source(&client, reference)?;
    let path = source.path.clone().unwrap_or(".amp.toml".into());
    let repo = utils::repo(&source.repo)?;

    let content = client
        .contents()
        .find(&repo, &path, &reference.rev())
        .map_err(|e| ResolveError::FetchingError(e.to_string()))?;
    debug!("The `.amp.toml` content of {} is:\n{:?}", repo, content);

    let manifest: Manifest = toml::from_slice(content.data.as_slice()).map_err(ResolveError::TomlParseFailed)?;
    load_from_manifest(credentials, manifest)
}
