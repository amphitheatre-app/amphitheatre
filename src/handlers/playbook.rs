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

use k8s_openapi::api::core::v1::Pod;
use kube::api::LogParams;
use kube::{Api, Client};
use rocket::futures::StreamExt;
use rocket::futures::TryStreamExt;
use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::Json;
use rocket::State;

use crate::database::Database;
use crate::models::playbook::Playbook;
use crate::services::playbook::PlaybookService;

/// The Playbooks Service Handlers.
/// See [API Documentation: playbook](https://docs.amphitheatre.app/api/playbook)

/// Lists the playbooks in the current account.
/// GET /v1/playbooks
#[get("/")]
pub async fn list(db: Database) -> Json<Vec<Playbook>> {
    let result = PlaybookService::list(&db).await;
    match result {
        Ok(playbooks) => Json(playbooks),
        Err(e) => Json(vec![]),
    }
}

/// Create a playbook in the current account.
/// POST /v1/playbooks
#[post("/")]
pub async fn create() -> Json<&'static str> {
    Json("OK")
}

/// Returns a playbook detail.
/// GET /v1/playbooks/<id>
#[get("/<id>")]
pub async fn detail(db: Database, id: u64) -> Json<Playbook> {
    let result = PlaybookService::get(&db, id).await;
    match result {
        Ok(playbook) => Json(playbook),
        Err(_) => Json(Playbook::default()),
    }
}

/// Update a playbook.
/// PATCH /v1/playbooks/<id>
#[patch("/<id>")]
pub async fn update(db: Database, id: u64) -> Json<&'static str> {
    Json("OK")
}

/// Delete a playbook
/// DELETE /v1/playbooks/<id>
#[delete("/<id>")]
pub async fn delete(db: Database, id: u64) -> Json<&'static str> {
    Json("OK")
}

/// Output the event streams of playbook
/// GET /v1/playbooks/<id>/events
#[get("/<id>/events")]
pub async fn events(client: &State<Client>, id: u64) -> EventStream![] {
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

/// Start a playbook.
/// POST /v1/playbooks/<id>/actions/start
#[post("/<id>/actions/start")]
pub async fn start(db: Database, id: u64) -> Json<&'static str> {
    Json("OK")
}

/// Stop a playbook.
/// POST /v1/playbooks/<id>/actions/stop
#[post("/<id>/actions/stop")]
pub async fn stop(db: Database, id: u64) -> Json<&'static str> {
    Json("OK")
}
