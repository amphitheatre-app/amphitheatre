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

use amp_common::config::{Credential, Credentials};
use amp_common::schema::{ActorSpec, GitReference};
use amp_common::scm::client::Client;
use tracing::debug;

use crate::errors::{ResolveError, Result};
use crate::utils;

pub fn source(client: &Client, source: &GitReference) -> Result<GitReference> {
    let mut actual = source.clone();

    // Return it if revision was provided.
    if actual.rev.is_some() {
        return Ok(actual);
    }

    let repo = utils::repo(&actual.repo)?;
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

pub fn image(credentials: &Credentials, spec: &ActorSpec) -> Result<String> {
    if !spec.image.is_empty() {
        return Ok(spec.image.clone());
    }

    // Generate image name based on the current registry and character name.
    if let Some(credential) = credentials.default_registry() {
        let mut registry = credential.server.as_str();
        if registry.eq("https://index.docker.io/v1/") {
            registry = "index.docker.io";
        }

        Ok(format!("{}/{}/{}", registry, credential.username_any(), spec.name))
    } else {
        Err(ResolveError::EmptyRegistryAddress)
    }
}
