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

mod build_ext;
pub use self::build_ext::BuildExt;

pub mod cluster_builder;
pub mod cluster_buildpack;
pub mod cluster_store;
pub mod image;
pub mod syncer;
pub mod types;

/// Encodes the image url to a valid Kubernetes resource name
/// e.g. "gcr.io/paketo-buildpacks/builder:base" -> "gcr-io-paketo-buildpacks-builder"
/// e.g. "gcr.io/paketo-buildpacks/java@sha256:fc1c6fba46b582f63b13490b89e50e93c95ce08142a8737f4a6b70c826c995de" -> "gcr-io-paketo-buildpacks-java"
pub fn encode_name(image: &str) -> String {
    let image = image.split('@').next().unwrap_or(image);
    let image = image.split(':').next().unwrap_or(image);

    image.replace(['.', '/'], "-").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::encode_name;

    #[test]
    fn test_encode_name() {
        assert_eq!(encode_name("gcr.io/paketo-buildpacks/builder:base"), "gcr-io-paketo-buildpacks-builder");
        assert_eq!(encode_name("gcr.io/paketo-buildpacks/builder"), "gcr-io-paketo-buildpacks-builder");
        assert_eq!(
            encode_name(
                "gcr.io/paketo-buildpacks/java@sha256:fc1c6fba46b582f63b13490b89e50e93c95ce08142a8737f4a6b70c826c995de"
            ),
            "gcr-io-paketo-buildpacks-java"
        );
    }
}
