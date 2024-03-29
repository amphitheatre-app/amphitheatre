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

use std::path::PathBuf;

use super::{docker_config_volume, git_sync, syncer, workspace_mount, workspace_volume, WORKSPACE_DIR};
use crate::args;
use crate::error::Result;

use amp_common::resource::{Actor, ActorSpec};
use k8s_openapi::api::core::v1::{Container, PodSpec, Volume, VolumeMount};

const DEFAULT_KANIKO_IMAGE: &str = "gcr.io/kaniko-project/executor:v1.15.0";

pub fn pod(actor: &Actor) -> Result<PodSpec> {
    // Choose the syncer for source code synchronization
    let syncer: Container;
    let mut volumes = vec![docker_config_volume(), workspace_volume()];
    if actor.spec.live {
        syncer = syncer::container(actor, &None)?;
    } else {
        syncer = git_sync::container(actor);
        volumes.push(git_source_volume());
    }

    Ok(PodSpec {
        init_containers: Some(vec![syncer]),
        containers: vec![container(&actor.spec)],
        restart_policy: Some("Never".into()),
        volumes: Some(volumes),
        ..Default::default()
    })
}

/// Build and return the container spec for the kaniko pod
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

    // TODO: Kaniko: Add support for multiple platforms in the future.
    // While Kaniko itself currently does not support creating multi-arch manifests,
    // See https://github.com/GoogleContainerTools/kaniko#creating-multi-arch-container-manifests-using-kaniko-and-manifest-tool
    // and https://github.com/GoogleContainerTools/kaniko#flag---custom-platform
    //
    // let custom_platform: String;
    // if let Some(platforms) = &build.platforms {
    //     custom_platform = platforms.join(",");
    //     arguments.push(("custom-platform", &custom_platform));
    // }

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
        volume_mounts: Some(vec![docker_config_mount(), workspace_mount()]),
        ..Default::default()
    }
}

/// Create a volume mount for the docker config
#[inline]
fn docker_config_mount() -> VolumeMount {
    VolumeMount { name: "docker-config".into(), mount_path: "/kaniko/.docker".into(), ..Default::default() }
}

fn git_source_volume() -> Volume {
    Volume { name: "src".to_string(), empty_dir: Some(Default::default()), ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kaniko_container() {
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
