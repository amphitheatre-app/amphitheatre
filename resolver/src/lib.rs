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

use amp_common::client::ClientError;
use amp_common::schema::{ActorSpec, Manifest, Source};
use amp_common::scm::client::Client;
use amp_common::scm::content::ContentService;
use amp_common::scm::driver::{github, Driver};
use amp_common::scm::git::GitService;
use amp_common::scm::repo::RepositoryService;
use thiserror::Error;
use tracing::debug;
use url::Url;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("ClientError: {0}")]
    ClientError(#[source] ClientError),

    #[error("InvalidRepository: {0}")]
    InvalidRepoAddress(#[source] url::ParseError),

    #[error("FetchingError: {0}")]
    FetchingError(String),

    #[error("TomlParseFailed: {0}")]
    TomlParseFailed(String),
}

pub type Result<T, E = ResolveError> = std::result::Result<T, E>;

/// Resolve the repo from the URL.
fn repo(url: &str) -> Result<String> {
    let url = Url::parse(url).map_err(ResolveError::InvalidRepoAddress)?;
    let mut repo = url.path().replace(".git", "");
    repo = repo.trim_start_matches('/').to_string();

    Ok(repo)
}

fn patch<T: Driver>(client: &Client<T>, source: &Source) -> Result<Source> {
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
pub fn load(source: &Source) -> Result<ActorSpec> {
    let client = Client::new(github::default());

    let source = patch(&client, source)?;
    let repo = repo(&source.repo)?;
    let path = source.path.clone().unwrap_or(".amp.toml".to_string());
    let content = client
        .conetnts()
        .find(&repo, &path, source.rev())
        .map_err(|e| ResolveError::FetchingError(e.to_string()))?;
    debug!("{:#?}", content);

    let manifest: Manifest = toml::from_slice(content.data.as_slice())
        .map_err(|e| ResolveError::TomlParseFailed(e.to_string()))?;

    let mut spec = ActorSpec::from(&manifest);
    spec.source = source;

    Ok(spec)
}
