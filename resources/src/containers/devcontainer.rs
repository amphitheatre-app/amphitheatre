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

use amp_common::resource::ActorSpec;
use k8s_openapi::api::core::v1::Container;

use super::{workspace_mount, DEFAULT_DEVCONTAINER_IMAGE};

/// Build and return the container spec for the devcontainer.
pub fn container(_spec: &ActorSpec) -> Container {
    Container {
        name: "builder".to_string(),

        // TODO: devcontainer image should be detected from actor spec,
        // it's parsed from the devcontainer.json file.
        // For now, we use the default image for testing.
        image: Some(DEFAULT_DEVCONTAINER_IMAGE.to_string()),
        // Use "command: ['sleep', 'infinity']" to keep the container running indefinitely.
        // Helpful for maintaining pod activity when no specific application logic is needed.
        // Not recommended for production; production containers should run actual applications.
        command: Some(vec!["sleep".into(), "infinity".into()]),
        image_pull_policy: Some("IfNotPresent".to_string()),
        volume_mounts: Some(vec![workspace_mount()]),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container() {
        let actor = ActorSpec { name: "test".into(), image: "test".into(), ..Default::default() };
        let container = container(&actor);

        assert_eq!(container.name, "builder");
        assert_eq!(container.image, Some(DEFAULT_DEVCONTAINER_IMAGE.to_string()));
        assert_eq!(container.image_pull_policy, Some("IfNotPresent".to_string()));
    }
}
