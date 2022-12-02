// Copyright 2022 The Amphitheatre Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;

use k8s_openapi::api::core::v1::Pod;
use kube::api::LogParams;
use kube::{Api, Client};
use rocket::futures::{StreamExt, TryStreamExt};
use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::Json;
use rocket::State;

use crate::database::Database;
use crate::models::actor::Actor;

/// The Actors Service Handlers.
/// See [API Documentation: playbook](https://docs.amphitheatre.app/api/actor)

/// Lists the actors of playbook.
/// GET /v1/playbooks/<pid>/actors
#[get("/<pid>/actors")]
pub async fn list(db: Database, pid: u64) -> Json<Vec<Actor>> {
    todo!()
}

/// Returns a actor detail.
/// GET /v1/actors/<id>
#[get("/<id>")]
pub async fn detail(db: Database, id: u64) -> Json<Actor> {
    todo!()
}

/// Output the log streams of actor
/// GET /v1/actors/<id>/logs
#[get("/<id>/logs")]
pub async fn logs(client: &State<Client>, id: u64) -> EventStream![] {
    let pods: Api<Pod> = Api::default_namespaced(client.inner().clone());
    let mut logs = pods
        .log_stream(
            "getting-started",
            &LogParams {
                follow: true,
                tail_lines: Some(1),
                ..LogParams::default()
            },
        )
        .await
        .unwrap()
        .boxed();

    EventStream! {
         while let Some(line) = logs.try_next().await.unwrap() {
             yield Event::data(format!("{:?}", String::from_utf8_lossy(&line)));
         }
    }
}

/// Returns a actor's info, including enviroments, volumes...
/// GET /v1/actors/<id>/info
#[get("/<id>/info")]
pub async fn info(id: u64) -> Json<HashMap<&'static str, HashMap<&'static str, &'static str>>> {
    Json(HashMap::from([
        (
            "environments",
            HashMap::from([
                ("K3S_TOKEN", "RdqNLMXRiRsHJhmxKurR"),
                ("K3S_KUBECONFIG_OUTPU", "/output/kubeconfig.yaml"),
                (
                    "PATH",
                    "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/bin/aux",
                ),
                (
                    "CRI_CONFIG_FILE",
                    "/var/lib/rancher/k3s/agent/etc/crictl.yaml",
                ),
            ]),
        ),
        ("mounts", HashMap::from([
            ("/VAR/LIB/CNI",
             "/var/lib/docker/volumes/00f49631b07ccd74de44d3047d5f889395ac871e05b622890b6dd788d34a59f4/_data"),
            ("/VAR/LIB/KUBELET",
             "/var/lib/docker/volumes/bc1b16d39a0e204841695de857122412cfdefd0f672af185b1fa43e635397848/_data"),
            ("/VAR/LIB/RANCHER/K3S",
             "/var/lib/docker/volumes/a78bcb9f7654701e0cfaef4447ef61ced4864e5b93dee7102ec639afb5cf2e1d/_data"),
            ("/VAR/LOG",
             "/var/lib/docker/volumes/f64c2f2cf81cfde89879f2a17924b31bd2f2e6a6a738f7df949bf6bd57102d25/_data"),
        ]
        )),
        ("port", HashMap::from([("6443/tcp", "0.0.0.0:42397")])),
    ]))
}

/// Returns a actor's stats.
/// GET /v1/actors/<id>/stats
#[get("/<id>/stats")]
pub async fn stats(id: u64) -> Json<HashMap<&'static str, &'static str>> {
    Json(HashMap::from([
        ("CPU USAGE", "1.98%"),
        ("MEMORY USAGE", "65.8MB"),
        ("DISK READ/WRITE", "5.3MB / 43.7 MB"),
        ("NETWORK I/O", "5.7 kB / 3 kB"),
    ]))
}
