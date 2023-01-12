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

use k8s_openapi::api::apps::v1::Deployment;
use kube::api::PostParams;
use kube::{Api, Client, ResourceExt};
use serde_json::{from_value, json};

use super::crds::{ActorSpec, Playbook};
use super::error::Result;
use crate::resources::error::Error;

pub async fn create(client: Client, playbook: &Playbook, actor: &ActorSpec) -> Result<Deployment> {
    let api: Api<Deployment> = Api::all(client);
    let params = PostParams::default();

    let mut deployment = from_value(json!({
      "apiVersion": "apps/v1",
      "kind": "Deployment",
      "metadata": {
        "name": actor.name,
        "labels": {
          "app.kubernetes.io/name": actor.name,
          "app.kubernetes.io/instance": playbook.name_any(),
          "app.kubernetes.io/managed-by": "Amphitheatre"
        }
      },
      "spec": {
        "replicas": 1,
        "selector": {
          "matchLabels": {
            "app.kubernetes.io/name": actor.name,
            "app.kubernetes.io/instance": playbook.name_any()
          }
        },
        "template": {
          "metadata": {
            "labels": {
              "app.kubernetes.io/name": actor.name,
              "app.kubernetes.io/instance": playbook.name_any()
            }
          },
          "spec": {
            "containers": [
              {
                "name": actor.name,
                "image": actor.image,
                "imagePullPolicy": "IfNotPresent",
              }
            ]
          }
        }
      }
    }))
    .map_err(Error::SerializationError)?;

    tracing::info!(
        "created deployment resource: {:#?}",
        serde_yaml::to_string(&deployment)
    );

    deployment = api
        .create(&params, &deployment)
        .await
        .map_err(Error::KubeError)?;

    Ok(deployment)
}
