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

use amp_common::schema::GitReference;
use k8s_openapi::api::core::v1::Container;

use super::{workspace_mount, DEFAULT_GIT_SYNC_IMAGE, WORKSPACE_DIR};
use crate::args;

pub fn container(source: &GitReference) -> Container {
    // Parse the arguments for the container
    let revision = source.rev();
    let arguments = vec![
        ("depth", "1"),
        ("one-time", "true"),
        ("ref", &revision),
        ("repo", &source.repo),
        ("root", "/workspace/src"),
        ("link", WORKSPACE_DIR),
    ];

    Container {
        name: "syncer".to_string(),
        image: Some(DEFAULT_GIT_SYNC_IMAGE.to_string()),
        image_pull_policy: Some("IfNotPresent".to_string()),
        args: Some(args(&arguments, 2)),
        volume_mounts: Some(vec![workspace_mount()]),
        ..Default::default()
    }
}
