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

use amp_common::resource::ActorSpec;
use k8s_openapi::api::core::v1::{Container, EnvVar, SecurityContext, VolumeMount};

use super::{workspace_mount, WORKSPACE_DIR};
use crate::args;

/// Build and return the container spec for the buildpacks container
pub fn container(spec: &ActorSpec, security_context: &Option<SecurityContext>) -> Container {
    let build = spec.character.build.clone().unwrap_or_default();

    // Parse the arguments for the container
    let arguments = vec![("app", WORKSPACE_DIR)];
    let mut arguments = args(&arguments, 1);
    if let Some(args) = &build.args {
        arguments.extend(args.clone());
    }
    arguments.push(spec.image.clone());

    // Parse the environment variables for the container
    let mut environment = vec![
        EnvVar { name: "CNB_PLATFORM_API".into(), value: Some("0.11".into()), ..Default::default() },
        EnvVar { name: "DOCKER_CONFIG".into(), value: Some("/workspace/.docker".into()), ..Default::default() },
    ];
    if let Some(env) = build.env() {
        environment.extend(env)
    }

    Container {
        name: "builder".to_string(),
        image: Some(build.buildpacks.unwrap_or_default().builder),
        command: Some(vec!["/cnb/lifecycle/creator".into()]),
        args: Some(arguments),
        env: Some(environment),
        volume_mounts: Some(vec![workspace_mount(), docker_config_mount()]),
        security_context: security_context.clone(),
        ..Default::default()
    }
}

/// Build and return the volume mount for the docker config
#[inline]
pub fn docker_config_mount() -> VolumeMount {
    VolumeMount { name: "docker-config".into(), mount_path: "/workspace/.docker".into(), ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container() {
        let spec = ActorSpec { name: "test".into(), image: "test".into(), ..Default::default() };

        let container = container(&spec, &None);

        assert_eq!(container.name, "builder");
        assert_eq!(container.image, Some("gcr.io/buildpacks/builder:v1".into()));
        assert_eq!(container.image_pull_policy, Some("IfNotPresent".into()));
        assert_eq!(container.command, Some(vec!["/cnb/lifecycle/creator".into()]));
        assert_eq!(container.args, Some(vec!["-app=/workspace/app".into(), "test".into()]));
        assert_eq!(
            container.env,
            Some(vec![
                EnvVar { name: "CNB_PLATFORM_API".into(), value: Some("0.11".into()), ..Default::default() },
                EnvVar { name: "DOCKER_CONFIG".into(), value: Some("/workspace/.docker".into()), ..Default::default() },
            ])
        );
        assert_eq!(
            container.volume_mounts,
            Some(vec![
                VolumeMount { name: "workspace".into(), mount_path: "/workspace".into(), ..Default::default() },
                VolumeMount {
                    name: "docker-config".into(),
                    mount_path: "/workspace/.docker".into(),
                    ..Default::default()
                },
            ])
        );
    }

    #[test]
    fn test_docker_config_mount() {
        let mount = docker_config_mount();

        assert_eq!(mount.name, "docker-config");
        assert_eq!(mount.mount_path, "/workspace/.docker");
    }
}
