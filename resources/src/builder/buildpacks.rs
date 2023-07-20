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
use k8s_openapi::api::core::v1::{Container, EnvVar, PodSpec};

use super::{git_sync, workspace_mount, workspace_volume};
use crate::args;

pub fn pod(spec: &ActorSpec) -> PodSpec {
    let service_account_name = std::env::var("AMP_SERVICE_ACCOUNT_NAME").unwrap_or("default".into());
    PodSpec {
        restart_policy: Some("Never".into()),
        init_containers: Some(vec![git_sync::container(&spec.source)]),
        containers: vec![container(spec)],
        service_account_name: Some(service_account_name),
        volumes: Some(vec![workspace_volume()]),
        ..Default::default()
    }
}

pub fn container(spec: &ActorSpec) -> Container {
    // Parse the arguments for the container
    let arguments = vec![("app", "/workspace/app")];
    let mut arguments = args(&arguments, 1);
    if let Some(argments) = spec.build_args() {
        arguments.extend(argments);
    }
    arguments.push(spec.docker_tag());

    // Parse the environment variables for the container
    let mut environment = vec![EnvVar {
        name: "CNB_PLATFORM_API".into(),
        value: Some("0.11".into()),
        ..Default::default()
    }];
    if let Some(env) = spec.build_env() {
        environment.extend(env)
    }

    Container {
        name: "builder".to_string(),
        image: Some(spec.builder()),
        image_pull_policy: Some("IfNotPresent".into()),
        command: Some(vec!["/cnb/lifecycle/creator".into()]),
        args: Some(arguments),
        env: Some(environment),
        volume_mounts: Some(vec![workspace_mount()]),
        ..Default::default()
    }
}
