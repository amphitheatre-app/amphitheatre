// Copyright 2023 The Amphitheatre Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Server;

use crate::context::Context;
use crate::{routes, swagger};

pub async fn run(ctx: Arc<Context>) {
    let port = ctx.config.port;

    let app = routes::build().merge(swagger::build()).with_state(ctx);

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let service = app.into_make_service_with_connect_info::<SocketAddr>();
    let server = Server::bind(&addr).serve(service).with_graceful_shutdown(async move {
        tokio::signal::ctrl_c().await.ok();
    });

    // Run this server for ... forever!
    if let Err(err) = server.await {
        tracing::error!("Server error: {}", err);
        std::process::exit(1)
    }
}
