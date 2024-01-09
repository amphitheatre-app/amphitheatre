// Copyright (c) The Amphitheatre Authors. All rights reserved.
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

use std::sync::Arc;

use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::context::Context;
use crate::handlers;

pub fn build() -> Router<Arc<Context>> {
    Router::new()
        // actors
        .route("/v1/actors/:pid/:name", get(handlers::actor::detail))
        .route("/v1/actors/:pid/:name/logs", get(handlers::actor::logs))
        .route("/v1/actors/:pid/:name/info", get(handlers::actor::info))
        .route("/v1/actors/:pid/:name/stats", get(handlers::actor::stats))
        .route("/v1/actors/:pid/:name/sync", post(handlers::actor::sync))
        //
        // playbooks
        .route("/v1/playbooks", get(handlers::playbook::list))
        .route("/v1/playbooks", post(handlers::playbook::create))
        .route("/v1/playbooks/:id", get(handlers::playbook::detail))
        .route("/v1/playbooks/:id", patch(handlers::playbook::update))
        .route("/v1/playbooks/:id", delete(handlers::playbook::delete))
        //
        .route("/v1/playbooks/:id/actions/start", post(handlers::playbook::start))
        .route("/v1/playbooks/:id/actions/stop", post(handlers::playbook::stop))
        .route("/v1/playbooks/:id/events", get(handlers::playbook::events))
        .route("/v1/playbooks/:id/actors", get(handlers::actor::list))
}
