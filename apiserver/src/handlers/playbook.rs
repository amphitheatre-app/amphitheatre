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

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Sse};
use axum::Json;
use futures::Stream;
use k8s_openapi::api::core::v1::Event as KEvent;
use kube::runtime::{watcher, WatchStreamExt};
use kube::Api;
use tokio_stream::StreamExt as _;
use uuid::Uuid;

use crate::context::Context;
use crate::requests::playbook::{CreatePlaybookRequest, UpdatePlaybookRequest};
use crate::response::{data, ApiError};
use crate::services::playbook::PlaybookService;

// The Playbooks Service Handlers.
// See [API Documentation: playbook](https://docs.amphitheatre.app/api/playbook)

/// Lists the playbooks in the current account.
#[utoipa::path(
    get, path = "/v1/playbooks",
    responses(
        (status = 200, description = "List all playbooks successfully", body = [PlaybookResponse]),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Playbooks"
)]
pub async fn list(State(ctx): State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    let playbooks = PlaybookService::list(ctx).await?;
    Ok(data(playbooks))
}

/// Create a playbook in the current account.
#[utoipa::path(
    post, path = "/v1/playbooks",
    request_body(
        content = inline(CreatePlaybookRequest),
        description = "Create playbook request",
        content_type = "application/json"
    ),
    responses(
        (status = 201, description = "Playbook created successfully", body = PlaybookResponse)
    ),
    tag = "Playbooks"
)]
pub async fn create(
    State(ctx): State<Arc<Context>>,
    Json(req): Json<CreatePlaybookRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let response = PlaybookService::create(ctx, &req).await?;
    Ok((StatusCode::CREATED, data(response)))
}

/// Returns a playbook detail.
#[utoipa::path(
    get, path = "/v1/playbooks/{id}",
    params(
        ("id" = Uuid, description = "The id of playbook"),
    ),
    responses(
        (status = 200, description = "Playbook found successfully", body = PlaybookResponse),
        (status = 404, description = "Playbook not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Playbooks"
)]
pub async fn detail(Path(id): Path<Uuid>, State(ctx): State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::get(ctx, id).await?;
    Ok(data(playbook))
}

/// Update a playbook.
#[utoipa::path(
    patch, path = "/v1/playbooks/{id}",
    params(
        ("id" = Uuid, description = "The id of playbook"),
    ),
    request_body(
        content = inline(UpdatePlaybookRequest),
        description = "Update playbook request",
        content_type = "application/json"
    ),
    responses(
        (status = 200, description = "Playbook updated successfully", body = PlaybookResponse),
        (status = 404, description = "Playbook not found")
    ),
    tag = "Playbooks"
)]
pub async fn update(
    Path(id): Path<Uuid>,
    State(ctx): State<Arc<Context>>,
    Json(payload): Json<UpdatePlaybookRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::update(ctx, id, payload.title, payload.description).await?;
    Ok(data(playbook))
}

/// Delete a playbook
#[utoipa::path(
    delete, path = "/v1/playbooks/{id}",
    params(
        ("id" = Uuid, description = "The id of playbook"),
    ),
    responses(
        (status = 204, description = "Playbook deleted successfully"),
        (status = 404, description = "Playbook not found")
    ),
    tag = "Playbooks"
)]
pub async fn delete(Path(id): Path<Uuid>, State(ctx): State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    PlaybookService::delete(ctx, id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Output the event streams of playbook
#[utoipa::path(
    get, path = "/v1/playbooks/{id}/events",
    params(
        ("id" = Uuid, description = "The id of playbook"),
    ),
    responses(
        (status = 200, description="Playbook's events found successfully"),
        (status = 404, description = "Playbook not found")
    ),
    tag = "Playbooks"
)]
pub async fn events(
    Path(id): Path<Uuid>,
    State(ctx): State<Arc<Context>>,
) -> Sse<impl Stream<Item = axum::response::Result<Event, Infallible>>> {
    let namespace = format!("amp-{}", id);
    let api: Api<KEvent> = Api::namespaced(ctx.k8s.clone(), namespace.as_str());
    let stream = watcher(api, watcher::Config::default())
        .applied_objects()
        .map(|result| match result {
            Ok(event) => Event::default().json_data(event).unwrap(),
            Err(err) => Event::default().event("error").data(err.to_string()),
        })
        .map(Ok)
        .throttle(Duration::from_secs(1));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Start a playbook.
#[utoipa::path(
    post, path = "/v1/playbooks/{id}/actions/start",
    params(
        ("id" = Uuid, description = "The id of playbook"),
    ),
    responses(
        (status = 204, description = "Playbook started successfully"),
        (status = 404, description = "Playbook not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Playbooks"
)]
pub async fn start(Path(id): Path<Uuid>, State(ctx): State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    PlaybookService::start(ctx, id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Stop a playbook.
#[utoipa::path(
    post, path = "/v1/playbooks/{id}/actions/stop",
    params(
        ("id" = Uuid, description = "The id of playbook"),
    ),
    responses(
        (status = 204, description = "Playbook stopped successfully"),
        (status = 404, description = "Playbook not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Playbooks",
)]
pub async fn stop(Path(id): Path<Uuid>, State(ctx): State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    PlaybookService::stop(ctx, id).await?;

    Ok(StatusCode::NO_CONTENT)
}
