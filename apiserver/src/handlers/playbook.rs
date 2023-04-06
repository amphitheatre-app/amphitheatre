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

use amp_common::schema::Source;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Sse};
use axum::Json;
use chrono::prelude::*;
use futures::Stream;
use k8s_openapi::api::core::v1::Event as KEvent;
use kube::api::ListParams;
use kube::runtime::{watcher, WatchStreamExt};
use kube::Api;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt as _;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::context::Context;
use crate::response::{data, ApiError};
use crate::services::playbook::PlaybookService;

// The Playbooks Service Handlers.
// See [API Documentation: playbook](https://docs.amphitheatre.app/api/playbook)

/// Lists the playbooks in the current account.
#[utoipa::path(
    get, path = "/v1/playbooks",
    responses(
        (status = 200, description = "List all playbooks successfully", body = [Playbook]),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Playbooks"
)]
pub async fn list(ctx: State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    let playbooks = PlaybookService::list(&ctx).await?;

    Ok(data(playbooks))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreatePlaybookRequest {
    pub title: String,
    pub description: String,
    pub preface: Source,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PlaybookResponse {
    /// The playbook ID in Amphitheatre.
    pub id: String,
    /// The title of the playbook.
    pub title: String,
    /// The description of the playbook.
    pub description: String,
    /// When the playbook was created in Amphitheatre.
    pub created_at: DateTime<Utc>,
    /// When the playbook was last updated in Amphitheatre.
    pub updated_at: DateTime<Utc>,
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
    ctx: State<Arc<Context>>,
    Json(req): Json<CreatePlaybookRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let response = PlaybookService::create(&ctx, &req).await?;
    Ok((StatusCode::CREATED, data(response)))
}

/// Returns a playbook detail.
#[utoipa::path(
    get, path = "/v1/playbooks/{id}",
    params(
        ("id" = Uuid, description = "The id of playbook"),
    ),
    responses(
        (status = 200, description = "Playbook found successfully", body = Playbook),
        (status = 404, description = "Playbook not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Playbooks"
)]
pub async fn detail(Path(id): Path<Uuid>, ctx: State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::get(&ctx, id).await?;

    match playbook {
        Some(playbook) => Ok(data(playbook)),
        None => Err(ApiError::NotFound),
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdatePlaybookRequest {
    title: Option<String>,
    description: Option<String>,
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
        (status = 200, description = "Playbook updated successfully", body = Playbook),
        (status = 404, description = "Playbook not found")
    ),
    tag = "Playbooks"
)]
pub async fn update(
    Path(id): Path<Uuid>,
    ctx: State<Arc<Context>>,
    Json(payload): Json<UpdatePlaybookRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::update(&ctx, id, payload.title, payload.description).await?;
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
pub async fn delete(Path(id): Path<Uuid>, ctx: State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::get(&ctx, id).await?;

    if playbook.is_none() {
        return Err(ApiError::NotFound);
    }

    PlaybookService::delete(&ctx, id).await?;
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
    ctx: State<Arc<Context>>,
) -> Sse<impl Stream<Item = axum::response::Result<Event, Infallible>>> {
    let namespace = format!("amp-{}", id);

    let api: Api<KEvent> = Api::namespaced(ctx.k8s.clone(), namespace.as_str());
    let params = ListParams::default();

    let stream = watcher(api, params)
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
pub async fn start(Path(id): Path<Uuid>, ctx: State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::get(&ctx, id).await?;

    if playbook.is_none() {
        return Err(ApiError::NotFound);
    }

    PlaybookService::start(&ctx, id).await?;
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
pub async fn stop(Path(id): Path<Uuid>, ctx: State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::get(&ctx, id).await?;

    if playbook.is_none() {
        return Err(ApiError::NotFound);
    }

    PlaybookService::stop(&ctx, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
