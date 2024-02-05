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

use super::{workspace_mount, WORKSPACE_DIR};
use crate::args;
use crate::error::{Error, Result};
use amp_common::resource::Actor;
use k8s_openapi::api::core::v1::{Container, SecurityContext};
use kube::ResourceExt;
use lazy_static::lazy_static;

// if release, use cargo pkg version, else use latest
lazy_static! {
    static ref DEFAULT_SYNCER_IMAGE: String = format!(
        "ghcr.io/amphitheatre-app/amp-syncer:{}",
        if cfg!(debug_assertions) { "latest" } else { env!("CARGO_PKG_VERSION") }
    );
}

/// Build and return the container spec for the syncer.
pub fn container(actor: &Actor, security_context: &Option<SecurityContext>) -> Result<Container> {
    let spec = &actor.spec;
    let playbook = owner_reference(actor)?;

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
        ("playbook", playbook.as_str()),
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
        security_context: security_context.clone(),
        ..Default::default()
    })
}

/// Get the playbook name from the owner reference.
#[inline]
fn owner_reference(actor: &Actor) -> Result<String> {
    actor
        .owner_references()
        .iter()
        .find_map(|owner| (owner.kind == "Playbook").then(|| owner.name.clone()))
        .ok_or_else(|| Error::MissingObjectKey(".metadata.ownerReferences"))
}

#[cfg(test)]
mod tests {
    use amp_common::resource::ActorSpec;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;

    use super::*;

    #[test]
    fn test_create_syncer_container() {
        let mut actor =
            Actor::new("test", ActorSpec { name: "test".into(), image: "test".into(), ..Default::default() });
        actor.metadata.owner_references = Some(vec![OwnerReference {
            api_version: "v1".into(),
            kind: "Playbook".into(),
            name: "test".into(),
            ..Default::default()
        }]);
        let container = container(&actor, &None).unwrap();

        assert_eq!(container.name, "syncer");
        assert_eq!(container.image, Some(DEFAULT_SYNCER_IMAGE.to_string()));
        assert_eq!(
            container.args,
            Some(vec![
                "--nats-url=nats://amp-nats.amp-system.svc:4222".into(),
                "--workspace=/workspace".into(),
                "--playbook=test".into(),
                "--actor=test".into(),
                "--once=false".into()
            ])
        );
    }
}
