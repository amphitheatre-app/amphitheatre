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
use std::time::Duration;

use axum::extract::Path;
use axum::response::sse::Event;
use axum::response::{IntoResponse, Sse};
use axum::{Extension, Json, TypedHeader};
use futures::{stream, Stream};
use tokio_stream::StreamExt as _;

use crate::database::Database;
use crate::models::playbook::Playbook;
use crate::services::playbook::PlaybookService;

/// The Playbooks Service Handlers.
/// See [API Documentation: playbook](https://docs.amphitheatre.app/api/playbook)

/// Lists the playbooks in the current account.
pub async fn list(Extension(db): Extension<Database>) -> impl IntoResponse {
    let result = PlaybookService::list(&db).await;
    match result {
        Ok(playbooks) => Json(playbooks),
        Err(e) => Json(vec![]),
    }
}

/// Create a playbook in the current account.
pub async fn create() -> impl IntoResponse {
    Json("OK")
}

/// Returns a playbook detail.
pub async fn detail(Path(id): Path<u64>, Extension(db): Extension<Database>) -> impl IntoResponse {
    let result = PlaybookService::get(&db, id).await;
    match result {
        Ok(playbook) => Json(playbook),
        Err(_) => Json(Playbook::default()),
    }
}

/// Update a playbook.
pub async fn update(Path(id): Path<u64>, Extension(db): Extension<Database>) -> impl IntoResponse {
    Json("OK")
}

/// Delete a playbook
pub async fn delete(Path(id): Path<u64>, Extension(db): Extension<Database>) -> impl IntoResponse {
    Json("OK")
}

/// Output the event streams of playbook
pub async fn events(
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

/// Start a playbook.
pub async fn start(Path(id): Path<u64>, Extension(db): Extension<Database>) -> impl IntoResponse {
    Json("OK")
}

/// Stop a playbook.
pub async fn stop(Path(id): Path<u64>, Extension(db): Extension<Database>) -> impl IntoResponse {
    Json("OK")
}
