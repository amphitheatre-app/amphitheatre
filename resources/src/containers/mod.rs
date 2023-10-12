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

// TODO: Using `/workspace` as the workspace directory.
const WORKSPACE_DIR: &str = "/workspace/app";

/// volume for /workspace based on k8s emptyDir
#[inline]
pub fn workspace_volume() -> Volume {
    Volume {
        name: "workspace".to_string(),
        empty_dir: Some(Default::default()),
        ..Default::default()
    }
}

/// volume mount for /workspace
#[inline]
pub fn workspace_mount() -> VolumeMount {
    VolumeMount {
        name: "workspace".to_string(),
        mount_path: "/workspace".to_string(),
        ..Default::default()
    }
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
