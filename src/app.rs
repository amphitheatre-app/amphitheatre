// Copyright 2022 The Amphitheatre Authors.
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

use axum::error_handling::HandleErrorLayer;
use axum::{BoxError, Server};
use tower::ServiceBuilder;
use tower_governor::errors::display_error;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;

use crate::context::Context;
use crate::{routes, swagger};

pub async fn run(ctx: Arc<Context>) {
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(1024)
            .burst_size(1024)
            .use_headers()
            .finish()
            .unwrap(),
    );

    let app = routes::build()
        .merge(swagger::build())
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(
                    |e: BoxError| async move { display_error(e) },
                ))
                .layer(GovernorLayer {
                    config: Box::leak(governor_conf),
                }),
        )
        .with_state(ctx);

    use tokio::signal::unix as usig;
    let mut shutdown = usig::signal(usig::SignalKind::terminate()).unwrap();

    // run it with hyper on localhost:3000
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let service = app.into_make_service_with_connect_info::<SocketAddr>();
    let server = Server::bind(&addr)
        .serve(service)
        .with_graceful_shutdown(async move {
            shutdown.recv().await;
        });

    // Run this server for ... forever!
    if let Err(err) = server.await {
        tracing::error!("Server error: {}", err);
        std::process::exit(1)
    }
}
