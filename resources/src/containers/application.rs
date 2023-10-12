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

use amp_common::resource::{Actor, ActorSpec};
use k8s_openapi::api::core::v1::{Container, PodSpec};

/// Build and return the pod spec for the actor
pub fn pod(actor: &Actor) -> PodSpec {
    PodSpec {
        containers: vec![container(&actor.spec)],
        ..Default::default()
    }
}

/// Build and return the container spec for the actor
fn container(spec: &ActorSpec) -> Container {
    let mut environments = Some(vec![]);
    let mut container_ports = Some(vec![]);

    // extract the env and ports from the deploy spec
    if let Some(deploy) = &spec.character.deploy {
        environments = deploy.env().clone();
        container_ports = deploy.container_ports();
    }

    Container {
        name: spec.name.clone(),
        image: Some(spec.image.clone()),
        image_pull_policy: Some("Always".into()),
        env: environments,
        ports: container_ports,
        ..Default::default()
    }
}
