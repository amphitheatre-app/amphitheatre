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

use std::path::PathBuf;

use amp_common::resource::ActorSpec;
use k8s_openapi::api::core::v1::{Container, VolumeMount};

use super::{workspace_mount, DEFAULT_KANIKO_IMAGE, WORKSPACE_DIR};
use crate::args;

/// Build and return the container spec for the kaniko container
pub fn container(spec: &ActorSpec) -> Container {
    let build = spec.character.build.clone().unwrap_or_default();

    // Set the working directory to context.
    let mut workdir = PathBuf::from(WORKSPACE_DIR);
    if let Some(context) = &build.context {
        workdir.push(context);
    }

    // Parse the arguments for the container
    let destination = spec.image.clone();
    let mut arguments = vec![
        ("context", workdir.to_str().unwrap()),
        ("destination", destination.as_str()),
        ("verbosity", "info"),
        ("cache", "true"),
    ];

    if let Some(config) = &build.dockerfile {
        arguments.push(("dockerfile", &config.dockerfile));
    }

    let mut arguments = args(&arguments, 2);
    if let Some(args) = &build.args {
        arguments.extend(args.clone());
    }

    Container {
        name: "builder".to_string(),
        image: Some(DEFAULT_KANIKO_IMAGE.into()),
        image_pull_policy: Some("IfNotPresent".into()),
        args: Some(arguments),
        env: build.env(),
        volume_mounts: Some(vec![workspace_mount(), docker_config_mount()]),
        ..Default::default()
    }
}

/// Build and return the volume mount for the kaniko docker config
#[inline]
fn docker_config_mount() -> VolumeMount {
    VolumeMount { name: "docker-config".into(), mount_path: "/kaniko/.docker".into(), ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container() {
        let spec = ActorSpec { name: "test".into(), image: "test".into(), ..Default::default() };

        let container = container(&spec);

        assert_eq!(container.name, "builder");
        assert_eq!(container.image, Some(DEFAULT_KANIKO_IMAGE.into()));
        assert_eq!(container.image_pull_policy, Some("IfNotPresent".into()));
    }

    #[test]
    fn test_docker_config_mount() {
        let mount = docker_config_mount();

        assert_eq!(mount.name, "docker-config");
        assert_eq!(mount.mount_path, "/kaniko/.docker");
    }
}
