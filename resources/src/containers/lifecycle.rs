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

use amp_common::resource::{Actor, ActorSpec};
use k8s_openapi::api::core::v1::SecurityContext;
use k8s_openapi::api::core::v1::{Container, EnvVar, PodSpec, VolumeMount};

use super::{docker_config_volume, git_sync, syncer, workspace_mount, workspace_volume, WORKSPACE_DIR};
use crate::args;

use crate::error::Result;

const DEFAULT_RUN_AS_GROUP: i64 = 1000;
const DEFAULT_RUN_AS_USER: i64 = 1001;

pub fn pod(actor: &Actor) -> Result<PodSpec> {
    // Get SecurityContext for the container
    let build = actor.spec.character.build.clone().unwrap_or_default();
    let builder = build.buildpacks.clone().unwrap_or_default().builder;
    let security_context = security_context(&builder);

    // Choose the syncer for source code synchronization
    let syncer =
        if actor.spec.live { syncer::container(actor, &security_context)? } else { git_sync::container(actor) };

    Ok(PodSpec {
        init_containers: Some(vec![syncer]),
        containers: vec![container(&actor.spec, &security_context)],
        restart_policy: Some("Never".into()),
        volumes: Some(vec![workspace_volume(), docker_config_volume()]),
        ..Default::default()
    })
}

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

/// Build SecurityContext for the container by Buildpacks builder mapping.
///
/// |user |group|builder|
/// |-----|-----|-------|
/// |1000 |1000 |heroku*|
/// |1000 |1000 |gcr.io/buildpacks*|
/// |1001 |1000 |paketobuildpacks*|
/// |1001 |1000 |amp-buildpacks*|
/// |1001 |1000 |*|
///
fn security_context(builder: &str) -> Option<SecurityContext> {
    let mut run_as_user = DEFAULT_RUN_AS_USER;

    if builder.starts_with("heroku") || builder.starts_with("gcr.io/buildpacks") {
        run_as_user = 1000;
    }

    Some(SecurityContext {
        run_as_user: Some(run_as_user),
        run_as_group: Some(DEFAULT_RUN_AS_GROUP),
        ..Default::default()
    })
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
