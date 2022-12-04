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

use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::handlers::*;

pub fn build() -> Router {
    Router::new()
        // actors
        .route("/v1/actors/:id", get(actor::detail))
        .route("/v1/actors/:id/logs", get(actor::logs))
        .route("/v1/actors/:id/info", get(actor::info))
        .route("/v1/actors/:id/stats", get(actor::stats))
        //
        // playbooks
        .route("/v1/playbooks", get(playbook::list))
        .route("/v1/playbooks", post(playbook::create))
        .route("/v1/playbooks/:id", get(playbook::detail))
        .route("/v1/playbooks/:id", patch(playbook::update))
        .route("/v1/playbooks/:id", delete(playbook::delete))
        //
        .route("/v1/playbooks/:id/actions/start", post(playbook::start))
        .route("/v1/playbooks/:id/actions/stop", post(playbook::stop))
        .route("/v1/playbooks/:id/events", get(playbook::events))
        .route("/v1/playbooks/:id/actors", get(actor::list))
}
