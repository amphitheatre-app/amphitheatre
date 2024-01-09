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

pub mod application;
pub mod buildpacks;
pub mod devcontainer;
pub mod git_sync;
pub mod kaniko;
pub mod syncer;

use k8s_openapi::api::core::v1::{KeyToPath, SecretVolumeSource, Volume, VolumeMount};

const DEFAULT_KANIKO_IMAGE: &str = "gcr.io/kaniko-project/executor:v1.15.0";
const DEFAULT_GIT_SYNC_IMAGE: &str = "registry.k8s.io/git-sync/git-sync:v4.0.0";
const DEFAULT_SYNCER_IMAGE: &str = "ghcr.io/amphitheatre-app/amp-syncer:latest";

/// This is the "universal" image that is used by default if no custom
/// Dockerfile or image is specified. Ubuntu-based default, large, and
/// multi-language universal image which contains many popular
/// languages/frameworks/SDKS/runtimes, lke Python, Node.js, JavaScript,
/// TypeScript, C++, Java, C#, F#, .NET Core, PHP, Go, Ruby, Conda. For
/// information about what's included in the default Linux image, see the
/// [devcontainers/images](https://github.com/devcontainers/images/tree/main/src/universal)
/// repository.
const DEFAULT_DEVCONTAINER_IMAGE: &str = "mcr.microsoft.com/devcontainers/universal:linux";

// TODO: Using `/workspace` as the workspace directory.
const WORKSPACE_DIR: &str = "/workspace/app";

/// volume for /workspace based on k8s emptyDir
#[inline]
pub fn workspace_volume() -> Volume {
    Volume { name: "workspace".to_string(), empty_dir: Some(Default::default()), ..Default::default() }
}

/// volume mount for /workspace
#[inline]
pub fn workspace_mount() -> VolumeMount {
    VolumeMount { name: "workspace".to_string(), mount_path: "/workspace".to_string(), ..Default::default() }
}

/// volume mount for docker config
#[inline]
pub fn docker_config_volume() -> Volume {
    Volume {
        name: "docker-config".to_string(),
        secret: Some(SecretVolumeSource {
            secret_name: Some("amp-registry-credentials".into()),
            items: Some(vec![KeyToPath {
                key: ".dockerconfigjson".into(),
                path: "config.json".into(),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_volume() {
        let volume = workspace_volume();

        assert_eq!(volume.name, "workspace");
        assert_eq!(volume.empty_dir, Some(Default::default()));
    }

    #[test]
    fn test_workspace_mount() {
        let mount = workspace_mount();

        assert_eq!(mount.name, "workspace");
        assert_eq!(mount.mount_path, "/workspace");
    }

    #[test]
    fn test_docker_config_volume() {
        let volume = docker_config_volume();

        assert_eq!(volume.name, "docker-config");

        let secret = volume.secret.unwrap();
        assert_eq!(secret.secret_name, Some("amp-registry-credentials".into()));

        let items = secret.items.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key, ".dockerconfigjson");
        assert_eq!(items[0].path, "config.json");
    }
}
