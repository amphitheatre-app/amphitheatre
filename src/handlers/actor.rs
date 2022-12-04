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
use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Extension, Path, TypedHeader};
use axum::response::sse::Event;
use axum::response::{IntoResponse, Json, Sse};
use futures::{stream, Stream};
use tokio_stream::StreamExt as _;

use crate::database::Database;

/// The Actors Service Handlers.
/// See [API Documentation: playbook](https://docs.amphitheatre.app/api/actor)

/// Lists the actors of playbook.
pub async fn list(Path(pid): Path<u64>, Extension(db): Extension<Database>) -> impl IntoResponse {
    todo!()
}

/// Returns a actor detail.
pub async fn detail(Path(id): Path<u64>, Extension(db): Extension<Database>) -> impl IntoResponse {
    todo!()
}

/// Output the log streams of actor
pub async fn logs(
    Path(id): Path<u64>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    println!("`{}` connected", user_agent.as_str());

    // A `Stream` that repeats an event every second
    let stream = stream::repeat_with(|| Event::default().data("hi!"))
        .map(Ok)
        .throttle(Duration::from_secs(1));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}

/// Returns a actor's info, including enviroments, volumes...
pub async fn info(Path(id): Path<u64>) -> impl IntoResponse {
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
pub async fn stats(Path(id): Path<u64>) -> impl IntoResponse {
    Json(HashMap::from([
        ("CPU USAGE", "1.98%"),
        ("MEMORY USAGE", "65.8MB"),
        ("DISK READ/WRITE", "5.3MB / 43.7 MB"),
        ("NETWORK I/O", "5.7 kB / 3 kB"),
    ]))
}
