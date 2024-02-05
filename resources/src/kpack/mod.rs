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

use crate::error::{Error, Result};
use amp_common::{
    config::{Credential, Credentials},
    resource::CharacterSpec,
};
use serde_json::json;
use sha2::{Digest, Sha256};

pub mod cluster_builder;
pub mod cluster_buildpack;
pub mod image;
pub mod syncer;

pub trait BuildExt {
    fn builder_name(&self) -> String;
    fn builder_items(&self) -> Option<Vec<String>>;
    fn builder_orders(&self) -> serde_json::Value;
    fn builder_tag(&self, credentials: &Credentials) -> Result<String>;
    fn pvc_name(&self) -> String;
}

impl BuildExt for CharacterSpec {
    /// Returns the name of the ClusterBuilder
    fn builder_name(&self) -> String {
        self.builder_items()
            .map(|items| format!("builder-{:x}", Sha256::digest(items.join(","))))
            .unwrap_or_else(|| "default-cluster-builder".to_string())
    }

    /// Returns the items of the builder
    fn builder_items(&self) -> Option<Vec<String>> {
        if let Some(config) = self.build.as_ref().and_then(|build| build.buildpacks.as_ref()) {
            let mut items = vec![];

            items.push(config.builder.clone());
            if let Some(buildpack) = &config.buildpacks {
                items.append(buildpack.clone().as_mut());
            }

            return Some(items);
        }

        None
    }

    /// Returns the orders of the builder
    fn builder_orders(&self) -> serde_json::Value {
        self.builder_items()
            .map(|buildpacks| {
                buildpacks
                    .iter()
                    .map(|item| json!({ "group": [{ "name": buildpack_name(item), "kind": "ClusterBuildpack" }] }))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns the tag of the builder image
    fn builder_tag(&self, credentials: &Credentials) -> Result<String> {
        // Generate image name based on the current registry and character name & revision
        if let Some(credential) = credentials.default_registry() {
            let mut registry = credential.server.as_str();
            if registry.eq("https://index.docker.io/v1/") {
                registry = "index.docker.io";
            }

            Ok(format!("{}/{}/{}", registry, credential.username_any(), self.builder_name()))
        } else {
            Err(Error::NotFoundRegistries)
        }
    }

    /// Returns the name of the PVC
    fn pvc_name(&self) -> String {
        format!("{}-pvc", self.meta.name)
    }
}

/// Parse docker image name to k8s metadata name
/// e.g. "gcr.io/paketo-buildpacks/builder:base" -> "gcr-io-paketo-buildpacks-builder"
/// e.g. "gcr.io/paketo-buildpacks/java@sha256:fc1c6fba46b582f63b13490b89e50e93c95ce08142a8737f4a6b70c826c995de" -> "gcr-io-paketo-buildpacks-java"
pub fn buildpack_name(image: &str) -> String {
    let image = image.split('@').next().unwrap_or(image);
    let image = image.split(':').next().unwrap_or(image);

    image.replace(['.', '/'], "-").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::buildpack_name;
    use super::BuildExt;

    use amp_common::{
        resource::CharacterSpec,
        schema::{Build, BuildpacksConfig, Metadata},
    };
    use serde_json::json;

    #[test]
    fn test_default_builder_name() {
        let character =
            CharacterSpec { build: Some(Build { buildpacks: None, ..Default::default() }), ..Default::default() };

        assert_eq!(character.builder_name(), "default-cluster-builder");
    }

    #[test]
    fn test_only_builder_name() {
        let character = CharacterSpec {
            build: Some(Build {
                buildpacks: Some(BuildpacksConfig { builder: "builder".to_string(), buildpacks: None }),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            character.builder_name(),
            "builder-df6b07176a9b17cc4c9afc257bd404732e7d09b76436c7890f7b7be14e579794"
        );
    }

    #[test]
    fn test_builder_name_with_buildpacks() {
        let character = CharacterSpec {
            build: Some(Build {
                buildpacks: Some(BuildpacksConfig {
                    builder: "builder".to_string(),
                    buildpacks: Some(vec!["buildpack1".to_string(), "buildpack2".to_string()]),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            character.builder_name(),
            "builder-d0ff8f7ef1e3dc23be46a919c8a4e1c660d516b46d72b704719f491a865c14f5"
        );
    }

    #[test]
    fn test_builder_items() {
        let character = CharacterSpec {
            build: Some(Build {
                buildpacks: Some(BuildpacksConfig {
                    builder: "builder".to_string(),
                    buildpacks: Some(vec!["buildpack1".to_string(), "buildpack2".to_string()]),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(character.builder_items().unwrap(), vec!["builder", "buildpack1", "buildpack2"]);
    }

    #[test]
    fn test_builder_orders() {
        let character = CharacterSpec {
            build: Some(Build {
                buildpacks: Some(BuildpacksConfig {
                    builder: "builder".to_string(),
                    buildpacks: Some(vec!["buildpack1".to_string(), "buildpack2".to_string()]),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            character.builder_orders(),
            json!([
                { "group": [{ "name": "builder", "kind": "ClusterBuildpack" }] },
                { "group": [{ "name": "buildpack1", "kind": "ClusterBuildpack" }] },
                { "group": [{ "name": "buildpack2", "kind": "ClusterBuildpack" }] }
            ])
        );
    }

    #[test]
    fn test_pvc_name() {
        let character = CharacterSpec {
            meta: Metadata { name: "amp-example-go".to_string(), ..Default::default() },
            ..Default::default()
        };

        assert_eq!(character.pvc_name(), "amp-example-go-pvc");
    }

    #[test]
    fn test_buildpack_name() {
        assert_eq!(buildpack_name("gcr.io/paketo-buildpacks/builder:base"), "gcr-io-paketo-buildpacks-builder");
        assert_eq!(buildpack_name("gcr.io/paketo-buildpacks/builder"), "gcr-io-paketo-buildpacks-builder");
        assert_eq!(
            buildpack_name(
                "gcr.io/paketo-buildpacks/java@sha256:fc1c6fba46b582f63b13490b89e50e93c95ce08142a8737f4a6b70c826c995de"
            ),
            "gcr-io-paketo-buildpacks-java"
        );
    }
}
