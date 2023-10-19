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

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use amp_common::sync::Synchronization;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Sse};
use axum::Json;

use futures::AsyncBufReadExt;
use futures::Stream;
use tokio_stream::StreamExt;

use k8s_openapi::api::core::v1::Pod;
use kube::api::LogParams;
use kube::Api;
use uuid::Uuid;

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
    let api: Api<Pod> = Api::namespaced(ctx.k8s.clone(), &pid.to_string());
    let params = LogParams::default();

    let stream = api
        .log_stream(&name, &params)
        .await
        .unwrap()
        .lines()
        .map(|result| match result {
            Ok(line) => Event::default().data(line),
            Err(err) => Event::default().event("error").data(err.to_string()),
        })
        .map(Ok)
        .throttle(Duration::from_secs(1));

    Sse::new(stream).keep_alive(KeepAlive::default())
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
    ActorService::sync(ctx, pid, name, req)
        .await
        .map_err(|err| ApiError::NatsError(err.to_string()))?;
    Ok(StatusCode::ACCEPTED)
}
