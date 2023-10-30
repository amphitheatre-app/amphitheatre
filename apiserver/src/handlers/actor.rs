// Copyright 2023 The Amphitheatre Authors.
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

use std::collections::HashSet;
use std::convert::Infallible;
use std::sync::Arc;

use amp_common::sync::Synchronization;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Sse};
use axum::Json;

use futures::{AsyncBufReadExt, Stream, StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::{ContainerStatus, Pod};
use kube::api::LogParams;
use kube::Api;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info};
use uuid::Uuid;

use kube::runtime::{watcher, WatchStreamExt};

use super::Result;
use crate::context::Context;
use crate::errors::ApiError;
use crate::services::actor::ActorService;

// The Actors Service Handlers.
// See [API Documentation: actor](https://docs.amphitheatre.app/api/actor)

/// Lists the actors of playbook.
#[utoipa::path(
    get, path = "/v1/playbooks/{pid}/actors",
    params(
        ("pid" = Uuid, description = "The id of playbook"),
    ),
    responses(
        (status = 200, description="List all actors of playbook successfully", body = [ActorResponse]),
        (status = 404, description = "Playbook not found")
    ),
    tag = "Actors"
)]
pub async fn list(Path(pid): Path<Uuid>, State(ctx): State<Arc<Context>>) -> Result<impl IntoResponse> {
    Ok(Json(ActorService::list(ctx, pid).await?))
}

/// Returns a actor detail.
#[utoipa::path(
    get, path = "/v1/actors/{pid}/{name}",
    params(
        ("pid" = Uuid, description = "The id of playbook"),
        ("name" = String, description = "The name of actor"),
    ),
    responses(
        (status = 200, description="Actor found successfully", body = ActorResponse),
        (status = 404, description = "Actor not found")
    ),
    tag = "Actors"
)]
pub async fn detail(
    State(ctx): State<Arc<Context>>,
    Path((pid, name)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse> {
    Ok(Json(ActorService::get(ctx, pid, name).await?))
}

/// Output the log streams of actor
#[utoipa::path(
    get, path = "/v1/actors/{pid}/{name}/logs",
    params(
        ("pid" = Uuid, description = "The id of playbook"),
        ("name" = String, description = "The name of actor"),
    ),
    responses(
        (status = 200, description="Actor's logs found successfully"),
        (status = 404, description = "Actor not found")
    ),
    tag = "Actors"
)]
pub async fn logs(
    State(ctx): State<Arc<Context>>,
    Path((pid, name)): Path<(Uuid, String)>,
) -> Sse<impl Stream<Item = axum::response::Result<Event, Infallible>>> {
    info!("Start to tail the log stream of actor {} in {}...", name, pid);
    let (sender, receiver) = tokio::sync::mpsc::channel(100);

    // Watch the status of the pod, if the pod is running, then create a stream for it.
    tokio::spawn(async move {
        let api: Api<Pod> = Api::namespaced(ctx.k8s.clone(), &format!("amp-{pid}"));
        let config = watcher::Config::default().labels(&format!("app.kubernetes.io/name={name}"));
        let mut watcher = watcher(api.clone(), config).applied_objects().boxed();
        let subs = Arc::new(RwLock::new(HashSet::new()));

        while let Some(pod) = watcher.try_next().await.unwrap() {
            if pod.status.is_none() {
                continue;
            }

            let status = pod.status.unwrap();
            let pod_name = pod.metadata.name.unwrap();

            // check the init container status, if it's not running, then skip it.
            if let Some(init_containers) = status.init_container_statuses {
                for status in init_containers {
                    log(&api, &pod_name, &status, &sender, subs.clone()).await;
                }
            }

            // check the container status, if it's not running, then skip it.
            if let Some(containers) = status.container_statuses {
                for status in containers {
                    log(&api, &pod_name, &status, &sender, subs.clone()).await;
                }
            }
        }
    });

    let stream = ReceiverStream::new(receiver);
    let stream = stream.map(|line| Event::default().data(line)).map(Ok);

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn log(
    api: &Api<Pod>,
    pod: &str,
    status: &ContainerStatus,
    sender: &Sender<String>,
    subs: Arc<RwLock<HashSet<String>>>,
) {
    let pod = pod.to_string();
    let name = status.name.clone();
    let subscription_id: String = format!("{pod}-{name}", pod = pod, name = name);

    debug!("container status: {:?}", status);

    // If the container is not running, skip it.
    if let Some(state) = &status.state {
        if state.running.is_none() {
            debug!("Skip log stream of container {} because it's not running.", name);
            return;
        }
    }
    // If job handle already exists in subscribe list, skip it.
    if subs.read().await.contains(&subscription_id) {
        debug!("Skip log stream of container {} because it's already subscribed.", name);
        return;
    }

    let api = api.clone();
    let sender = sender.clone();

    tokio::spawn(async move {
        let params = LogParams { container: Some(name.clone()), follow: true, timestamps: true, ..Default::default() };
        let mut stream =
            api.log_stream(&pod, &params).await.map_err(|e| ApiError::KubernetesError(e.to_string())).unwrap().lines();

        info!("Start to receive the log stream of container {} in {}...", name, pod);
        while let Some(line) = stream.try_next().await.unwrap() {
            let _ = sender.send(line).await;
        }
    });

    // save the job handle to subscribe list.
    subs.write().await.insert(subscription_id);
}

/// Returns a actor's info, including environments, volumes...
#[utoipa::path(
    get, path = "/v1/actors/{pid}/{name}/info",
    params(
        ("pid" = Uuid, description = "The id of playbook"),
        ("name" = String, description = "The name of actor"),
    ),
    responses(
        (status = 200, description="Actor's info found successfully"),
        (status = 404, description = "Actor not found")
    ),
    tag = "Actors"
)]
pub async fn info(
    State(ctx): State<Arc<Context>>,
    Path((pid, name)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse> {
    Ok(Json(ActorService::info(ctx, pid, name).await?))
}

/// Returns a actor's stats.
#[utoipa::path(
    get, path = "/v1/actors/{pid}/{name}/stats",
    params(
        ("pid" = Uuid, description = "The id of playbook"),
        ("name" = String, description = "The name of actor"),
    ),
    responses(
        (status = 200, description="Actor's stats found successfully"),
        (status = 404, description = "Actor not found")
    ),
    tag = "Actors"
)]
pub async fn stats(
    State(ctx): State<Arc<Context>>,
    Path((pid, name)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse> {
    Ok(Json(ActorService::stats(ctx, pid, name).await?))
}

/// Receive a actor's sources and publish them to Message Queue.
#[utoipa::path(
    post, path = "/v1/actors/{pid}/{name}/sync",
    params(
        ("pid" = Uuid, description = "The id of playbook"),
        ("name" = String, description = "The name of actor"),
    ),
    request_body(
        content = inline(Synchronization),
        description = "File synchronization request body",
        content_type = "application/json"
    ),
    responses(
        (status = 200, description="Sync the actor's sources successfully"),
        (status = 404, description = "Actor not found")
    ),
    tag = "Actors"
)]
pub async fn sync(
    State(ctx): State<Arc<Context>>,
    Path((pid, name)): Path<(Uuid, String)>,
    Json(req): Json<Synchronization>,
) -> Result<impl IntoResponse> {
    ActorService::sync(ctx, pid, name, req).await.map_err(|err| ApiError::NatsError(err.to_string()))?;
    Ok(StatusCode::ACCEPTED)
}
