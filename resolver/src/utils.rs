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

use url::Url;

use crate::errors::{ResolveError, Result};

/// Resolve the repo from the URL.
pub fn repo(url: &str) -> Result<String> {
    let url = Url::parse(url).map_err(ResolveError::InvalidRepoAddress)?;
    let mut repo = url.path().replace(".git", "");
    repo = repo.trim_start_matches('/').to_string();

    Ok(repo)
}
