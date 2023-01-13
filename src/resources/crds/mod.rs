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

mod actor;
mod playbook;

pub use self::actor::*;
pub use self::playbook::*;

fn url(repository: &String, reference: &Option<String>, path: &Option<String>) -> String {
    let mut url = String::from(repository);

    if reference.is_some() || path.is_some() {
        url.push('#');
    }

    if let Some(reference) = reference {
        url.push_str(reference.as_str());
    }

    if let Some(path) = path {
        url.push(':');
        url.push_str(path.as_str());
    }

    url
}
