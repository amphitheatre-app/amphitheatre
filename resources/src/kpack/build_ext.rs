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
    schema::BuildpacksConfig,
};
use sha2::{Digest, Sha256};

use super::encode_name;

pub trait BuildExt {
    fn builder_name(&self) -> String;
    fn buildpacks(&self) -> Option<&Vec<String>>;
    // fn builder_orders(&self) -> serde_json::Value;
    fn builder_tag(&self, credentials: &Credentials) -> Result<String>;
    fn store_name(&self) -> String;
    fn store_image(&self) -> String;
    fn pvc_name(&self) -> String;

    fn get_buildpacks_config(&self) -> Option<&BuildpacksConfig>;
}

impl BuildExt for CharacterSpec {
    /// Returns the name of the ClusterBuilder
    fn builder_name(&self) -> String {
        self.get_buildpacks_config()
            .map(|config| match &config.buildpacks {
                Some(buildpacks) => {
                    let mut items = vec![];
                    items.push(config.builder.clone());
                    items.append(buildpacks.clone().as_mut());
                    format!("builder-{:x}", Sha256::digest(items.join(",").as_bytes()))
                }
                None => encode_name(&config.builder),
            })
            .unwrap_or_else(|| "default-cluster-builder".to_string())
    }

    /// Returns the buildpacks items
    #[inline]
    fn buildpacks(&self) -> Option<&Vec<String>> {
        self.get_buildpacks_config().and_then(|config| config.buildpacks.as_ref())
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

    /// Returns the name of the ClusterBuilder
    fn store_name(&self) -> String {
        self.get_buildpacks_config()
            .map(|config| encode_name(&config.builder))
            .unwrap_or_else(|| "default-cluster-store".to_string())
    }

    /// Returns the image of the ClusterStore
    fn store_image(&self) -> String {
        self.get_buildpacks_config()
            .map(|config| config.builder.clone())
            .unwrap_or_else(|| "default-cluster-store".to_string())
    }

    /// Returns the name of the PVC
    fn pvc_name(&self) -> String {
        format!("{}-pvc", self.meta.name)
    }

    /// Returns the buildpacks config
    #[inline]
    fn get_buildpacks_config(&self) -> Option<&BuildpacksConfig> {
        self.build.as_ref().and_then(|build| build.buildpacks.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::BuildExt;

    use amp_common::config::Credentials;
    use amp_common::config::RegistryCredential;
    use amp_common::{
        resource::CharacterSpec,
        schema::{Build, BuildpacksConfig, Metadata},
    };

    #[test]
    fn test_default_builder_name() {
        let character = CharacterSpec::default();
        assert_eq!(character.builder_name(), "default-cluster-builder");
    }

    #[test]
    fn test_only_builder_name() {
        let character = CharacterSpec {
            build: Some(Build {
                buildpacks: Some(BuildpacksConfig {
                    builder: "amp-buildpacks/sample-builder:v1".to_string(),
                    buildpacks: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(character.builder_name(), "amp-buildpacks-sample-builder");
    }

    #[test]
    fn test_builder_name_with_buildpacks() {
        let character = CharacterSpec {
            build: Some(Build {
                buildpacks: Some(BuildpacksConfig {
                    builder: "builder".to_string(),
                    buildpacks: Some(vec![
                        "amp-buildpacks/buildpack1".to_string(),
                        "amp-buildpacks/buildpack2".to_string(),
                    ]),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            character.builder_name(),
            "builder-08190d5ac2f0b4e061adc39255c82f0cef6dbac27ffb0eadb8cc3bb7f8bb6eae"
        );
    }

    #[test]
    fn test_buildpacks() {
        let character = CharacterSpec {
            build: Some(Build {
                buildpacks: Some(BuildpacksConfig {
                    builder: "builder".to_string(),
                    buildpacks: Some(vec![
                        "amp-buildpacks/buildpack1".to_string(),
                        "amp-buildpacks/buildpack2".to_string(),
                    ]),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            character.buildpacks(),
            Some(&vec!["amp-buildpacks/buildpack1".to_string(), "amp-buildpacks/buildpack2".to_string()])
        );
    }

    #[test]
    fn test_builder_tag() {
        let character = CharacterSpec {
            build: Some(Build {
                buildpacks: Some(BuildpacksConfig {
                    builder: "amp-buildpacks/sample-builder:v1".to_string(),
                    buildpacks: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let credentials = Credentials {
            registries: vec![RegistryCredential {
                name: "docker.io".to_string(),
                default: true,
                server: "https://index.docker.io/v1/".to_string(),
                username: Some("user".to_string()),
                password: Some("password".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };

        assert_eq!(character.builder_tag(&credentials).unwrap(), "index.docker.io/user/amp-buildpacks-sample-builder");
    }

    #[test]
    fn test_store_name() {
        let character = CharacterSpec {
            build: Some(Build {
                buildpacks: Some(BuildpacksConfig {
                    builder: "amp-buildpacks/sample-builder:v1".to_string(),
                    buildpacks: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(character.store_name(), "amp-buildpacks-sample-builder");
    }

    #[test]
    fn test_store_image() {
        let character = CharacterSpec {
            build: Some(Build {
                buildpacks: Some(BuildpacksConfig {
                    builder: "amp-buildpacks/sample-builder:v1".to_string(),
                    buildpacks: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(character.store_image(), "amp-buildpacks/sample-builder:v1");
    }

    #[test]
    fn test_pvc_name() {
        let character = CharacterSpec {
            meta: Metadata { name: "amp-example-go".to_string(), ..Default::default() },
            ..Default::default()
        };

        assert_eq!(character.pvc_name(), "amp-example-go-pvc");
    }
}
