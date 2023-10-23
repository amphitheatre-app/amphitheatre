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

use amp_common::resource::CharacterSpec;
use amp_common::schema::{Character, GitReference};
use amp_common::scm::client::Client as ScmClient;
use amp_common::{config::Credentials, resource::ActorSpec};
use amp_resources::character;
use errors::{ResolveError, Result};
use kube::Client as KubeClient;
use tracing::debug;

pub mod errors;
pub mod partner;
pub mod patches;
pub mod preface;
pub mod utils;

const CATALOG_REPO_URL: &str = "https://github.com/amphitheatre-app/catalog.git";

/// Load manifest from catalog and return the actor spec.
pub fn load_from_catalog(credentials: &Credentials, name: &str, version: &str) -> Result<CharacterSpec> {
    let reference = GitReference {
        repo: CATALOG_REPO_URL.to_string(),
        path: Some(format!("characters/{}/{}/amp.toml", name, version)),
        ..GitReference::default()
    };
    debug!("Loading character from catalog: {:?}", reference);
    load_from_source(credentials, &reference)
}

/// Load manifest from remote VCS (like github) and return the actor spec.
pub fn load_from_source(credentials: &Credentials, reference: &GitReference) -> Result<CharacterSpec> {
    let client = ScmClient::init(credentials, &reference.repo).map_err(ResolveError::SCMError)?;

    let reference = patches::source(&client, reference)?;
    let path = reference.path.clone().unwrap_or(".amp.toml".into());
    let repo = utils::repo(&reference.repo)?;

    let content = client
        .contents()
        .find(&repo, &path, &reference.rev())
        .map_err(|e| ResolveError::FetchingError(e.to_string()))?;
    let data = std::str::from_utf8(&content.data).map_err(ResolveError::ConvertBytesError)?;
    debug!("The `.amp.toml` content of {} is:\n{:?}", repo, data);

    let manifest: Character = toml::from_str(data).map_err(ResolveError::TomlParseFailed)?;
    Ok(CharacterSpec::from(manifest))
}

/// Load manifest from Kubernetes cluster and return the actor spec.
pub async fn load_from_cluster(client: &KubeClient, name: &str) -> Result<CharacterSpec> {
    let character = character::get(client, name).await.map_err(ResolveError::ResourceError)?;
    Ok(character.spec)
}

/// Read Character manifest and return the actor spec.
pub fn to_actor(character: &CharacterSpec, credentials: &Credentials) -> Result<ActorSpec> {
    let client = ScmClient::init(credentials, &character.meta.repository).map_err(ResolveError::SCMError)?;

    let mut actor = ActorSpec::from(character);

    // Patch the source and image if the actor is not live.
    // it will be build with the builders later, so these must be valid.
    if !actor.live {
        actor.source = Some(patches::source(&client, &actor.source.unwrap())?);
        actor.image = patches::image(credentials, &actor)?;
    } else {
        // Remove the source if the actor is live.
        // it will be sync with the syncer later, so this is not necessary.
        actor.source = None;
    }

    Ok(actor)
}
