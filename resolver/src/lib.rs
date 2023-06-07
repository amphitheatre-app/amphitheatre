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

use amp_common::config::{Credential, CredentialConfiguration};
use amp_common::schema::{ActorSpec, Manifest, Source};
use amp_common::scm::client::Client;
use errors::{ResolveError, Result};
use tracing::debug;
use url::Url;

pub mod errors;

/// Resolve the repo from the URL.
fn repo(url: &str) -> Result<String> {
    let url = Url::parse(url).map_err(ResolveError::InvalidRepoAddress)?;
    let mut repo = url.path().replace(".git", "");
    repo = repo.trim_start_matches('/').to_string();

    Ok(repo)
}

fn patch(client: &Client, source: &Source) -> Result<Source> {
    let mut actual = source.clone();

    // Return it if revision was provided.
    if actual.rev.is_some() {
        return Ok(actual);
    }

    let repo = repo(&actual.repo)?;
    let reference: String;

    debug!("the repo name parsed from repository address is: {}", repo);

    // Check which reference is primary (branch or tag),
    // prioritize the tag first, then the branch,
    // otherwise read the remote repository and get its default branch
    if let Some(tag) = &actual.tag {
        reference = tag.to_string();
    } else if let Some(branch) = &actual.branch {
        reference = branch.to_string();
    } else {
        let repository = client
            .repositories()
            .find(&repo)
            .map_err(|e| ResolveError::FetchingError(e.to_string()))?;
        reference = repository.unwrap().branch;

        // Save it for other purposes,
        // such as a reference value when re-modifying
        actual.branch = Some(reference.to_string());
    }

    // Get its real latest revision according to the reference
    let commit = client.git().find_commit(&repo, &reference).unwrap();
    actual.rev = Some(commit.unwrap().sha);

    Ok(actual)
}

/// Read real actor information from remote VCS (like github).
pub fn load(configuration: &CredentialConfiguration, source: &Source) -> Result<ActorSpec> {
    // Initialize the client by source host.
    let client = Client::init(configuration, source).map_err(ResolveError::SCMError)?;
    let source = patch(&client, source)?;
    let repo = repo(&source.repo)?;
    let path = source.path.clone().unwrap_or(".amp.toml".into());

    let content = client
        .contents()
        .find(&repo, &path, source.rev())
        .map_err(|e| ResolveError::FetchingError(e.to_string()))?;
    debug!("The `.amp.toml` content of {} is:\n{:?}", repo, content);

    let manifest: Manifest =
        toml::from_slice(content.data.as_slice()).map_err(|e| ResolveError::TomlParseFailed(e.to_string()))?;

    let mut spec = ActorSpec::from(&manifest);
    spec.source = source;

    // Generate image name based on the current registry and character name.
    if manifest.character.image.is_none() {
        if let Some(credential) = configuration.default_registry() {
            let mut registry = credential.server.as_str();
            if registry.eq("https://index.docker.io/v1/") {
                registry = "index.docker.io";
            }

            spec.image = format!("{}/{}/{}", registry, credential.username_any(), spec.name);
        } else {
            return Err(ResolveError::EmptyRegistryAddress);
        }
    }

    Ok(spec)
}
