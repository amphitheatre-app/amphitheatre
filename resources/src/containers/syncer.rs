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
use k8s_openapi::api::core::v1::Container;

use super::{workspace_mount, DEFAULT_SYNCER_IMAGE, WORKSPACE_DIR};
use crate::args;
use crate::error::Result;

/// Build and return the container spec for the syncer.
pub fn container(playbook: &str, spec: &ActorSpec) -> Result<Container> {
    // Set the working directory to `workspace` argument.
    let build = spec.character.build.clone().unwrap_or_default();
    let mut workdir = PathBuf::from(WORKSPACE_DIR);
    if let Some(context) = &build.context {
        workdir.push(context);
    }

    // FIXME: get the nats url from the config of context.
    let once = spec.once.to_string();
    let arguments = vec![
        ("nats-url", "nats://amp-nats.amp-system.svc:4222"),
        ("workspace", workdir.to_str().unwrap()),
        ("playbook", playbook),
        ("actor", spec.name.as_str()),
        ("once", once.as_str()),
    ];

    // FIXME: get the syncer image from the config of context.
    Ok(Container {
        name: "syncer".to_string(),
        image: Some(DEFAULT_SYNCER_IMAGE.to_string()),
        command: Some(vec!["/usr/local/bin/amp-syncer".into()]),
        // image_pull_policy: Some("IfNotPresent".to_string()),
        args: Some(args(&arguments, 2)),
        volume_mounts: Some(vec![workspace_mount()]),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container() {
        let spec = ActorSpec { name: "test".into(), image: "test".into(), ..Default::default() };
        let container = container("test", &spec).unwrap();

        assert_eq!(container.name, "syncer");
        assert_eq!(container.image, Some(DEFAULT_SYNCER_IMAGE.into()));
        assert_eq!(
            container.args,
            Some(vec![
                "--nats-url=nats://amp-nats.amp-system.svc:4222".into(),
                "--workspace=/workspace/app".into(),
                "--playbook=test".into(),
                "--actor=test".into(),
                "--once=false".into()
            ])
        );
    }
}
