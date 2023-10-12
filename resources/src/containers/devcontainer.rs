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

use amp_common::resource::Actor;
use k8s_openapi::api::core::v1::{Container, PodSpec};

use super::{syncer, workspace_mount, workspace_volume};
use crate::error::Result;

/// Build and return the pod spec for the devcontainer build deployment
pub fn pod(actor: &Actor) -> Result<PodSpec> {
    let syncer = syncer::container(actor)?;
    let builder = container(actor);

    Ok(PodSpec {
        containers: vec![syncer, builder],
        volumes: Some(vec![workspace_volume()]),
        ..Default::default()
    })
}

/// Build and return the container spec for the devcontainer.
fn container(_actor: &Actor) -> Container {
    // TODO: devcontainer image should be detected from actor spec,
    // it's parsed from the devcontainer.json file.
    // For now, we use the default image for testing.
    let image = "mcr.microsoft.com/devcontainers/go";

    Container {
        name: "builder".to_string(),
        image: Some(image.to_string()),
        image_pull_policy: Some("IfNotPresent".to_string()),
        volume_mounts: Some(vec![workspace_mount()]),
        ..Default::default()
    }
}
