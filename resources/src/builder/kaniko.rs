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

use amp_common::schema::ActorSpec;
use k8s_openapi::api::core::v1::{Container, KeyToPath, PodSpec, SecretVolumeSource, Volume, VolumeMount};

use super::{git_sync, DEFAULT_KANIKO_IMAGE};
use crate::args;

pub fn pod(spec: &ActorSpec) -> PodSpec {
    PodSpec {
        restart_policy: Some("Never".into()),
        init_containers: Some(vec![git_sync::container(&spec.source)]),
        containers: vec![container(spec)],
        volumes: Some(vec![kaniko_secret_volume(), kaniko_secret_volume()]),
        ..Default::default()
    }
}

pub fn container(spec: &ActorSpec) -> Container {
    // Parse the arguments for the container
    let destination = spec.docker_tag();
    let arguments = vec![
        ("context", "/workspace/app"),
        ("destination", destination.as_str()),
        ("verbosity", "trace"),
        ("cache", "true"),
    ];
    let mut arguments = args(&arguments, 2);
    if let Some(argments) = spec.build_args() {
        arguments.extend(argments);
    }

    Container {
        name: "builder".to_string(),
        image: Some(DEFAULT_KANIKO_IMAGE.into()),
        image_pull_policy: Some("IfNotPresent".into()),
        args: Some(arguments),
        env: spec.build_env(),
        volume_mounts: Some(vec![kaniko_secret_mount()]),
        ..Default::default()
    }
}

#[inline]
fn kaniko_secret_volume() -> Volume {
    Volume {
        name: "kaniko-secret".to_string(),
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

#[inline]
fn kaniko_secret_mount() -> VolumeMount {
    VolumeMount {
        name: "kaniko-secret".into(),
        mount_path: "/kaniko/.docker".into(),
        ..Default::default()
    }
}
