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

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::Event;
use axum::response::{IntoResponse, Sse};
use axum::{Json, TypedHeader};
use futures::{stream, Stream};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt as _;
use utoipa::ToSchema;

use crate::app::Context;
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
    )
)]
pub async fn list(ctx: State<Arc<Context>>) -> Result<impl IntoResponse, ApiError> {
    let playbooks = PlaybookService::list(&ctx.db).await?;

    Ok(data(playbooks))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreatePlaybookRequest {
    title: String,
    description: String,
}

/// Create a playbook in the current account.
#[utoipa::path(
    post, path = "/v1/playbooks",
    request_body(
        content = inline(CreatePlaybookRequest),
        description = "create playbook request",
        content_type = "application/json"
    ),
    responses(
        (status = 201, description = "Playbook created successfully", body = Playbook)
    )
)]
pub async fn create(
    ctx: State<Arc<Context>>,
    Json(payload): Json<CreatePlaybookRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::create(ctx, payload.title, payload.description).await?;
    Ok((StatusCode::CREATED, data(playbook)))
}

/// Returns a playbook detail.
#[utoipa::path(
    get, path = "/v1/playbooks/{id}",
    params(
        ("id", description = "The id of playbook"),
    ),
    responses(
        (status = 200, description = "Playbook found successfully", body = Playbook),
        (status = 404, description = "Playbook not found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
pub async fn detail(
    Path(id): Path<u64>,
    ctx: State<Arc<Context>>,
) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::get(&ctx.db, id).await?;

    match playbook {
        Some(playbook) => Ok(data(playbook)),
        None => Err(ApiError::NotFound),
    }
}

/// Update a playbook.
#[utoipa::path(
    patch, path = "/v1/playbooks/{id}",
    params(
        ("id", description = "The id of playbook"),
    ),
    responses(
        (status = 200, description = "Playbook updated successfully", body = Playbook),
        (status = 404, description = "Playbook not found")
    )
)]
pub async fn update(Path(id): Path<u64>, ctx: State<Arc<Context>>) -> impl IntoResponse {
    Json("OK")
}

/// Delete a playbook
#[utoipa::path(
    delete, path = "/v1/playbooks/{id}",
    params(
        ("id", description = "The id of playbook"),
    ),
    responses(
        (status = 204, description = "Playbook deleted successfully"),
        (status = 404, description = "Playbook not found")
    )
)]
pub async fn delete(
    Path(id): Path<u64>,
    ctx: State<Arc<Context>>,
) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::get(&ctx.db, id).await?;

    if playbook.is_none() {
        return Err(ApiError::NotFound);
    }

    PlaybookService::delete(&ctx.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Output the event streams of playbook
#[utoipa::path(
    get, path = "/v1/playbooks/{id}/events",
    params(
        ("id", description = "The id of playbook"),
    ),
    responses(
        (status = 200, description="Playbook's events found successfully"),
        (status = 404, description = "Playbook not found")
    )
)]
pub async fn events(
    Path(id): Path<u64>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Sse<impl Stream<Item = axum::response::Result<Event, Infallible>>> {
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

/// Start a playbook.
#[utoipa::path(
    post, path = "/v1/playbooks/{id}/actions/start",
    params(
        ("id", description = "The id of playbook"),
    ),
    responses(
        (status = 204, description = "Playbook started successfully"),
        (status = 404, description = "Playbook not found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
pub async fn start(
    Path(id): Path<u64>,
    ctx: State<Arc<Context>>,
) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::get(&ctx.db, id).await?;

    if playbook.is_none() {
        return Err(ApiError::NotFound);
    }

    PlaybookService::start(&ctx.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Stop a playbook.
#[utoipa::path(
    post, path = "/v1/playbooks/{id}/actions/stop",
    params(
        ("id", description = "The id of playbook"),
    ),
    responses(
        (status = 204, description = "Playbook stopped successfully"),
        (status = 404, description = "Playbook not found"),
        (status = 500, description = "Internal Server Error"),
    )
)]
pub async fn stop(
    Path(id): Path<u64>,
    ctx: State<Arc<Context>>,
) -> Result<impl IntoResponse, ApiError> {
    let playbook = PlaybookService::get(&ctx.db, id).await?;

    if playbook.is_none() {
        return Err(ApiError::NotFound);
    }

    PlaybookService::stop(&ctx.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
