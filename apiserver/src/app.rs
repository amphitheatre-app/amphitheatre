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

use crate::context::Context;
use crate::{routes, swagger};

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub async fn run(ctx: Arc<Context>) {
    let port = ctx.config.port;

    // build our application with a route
    let app = routes::build().merge(swagger::build()).with_state(ctx).layer((
        TraceLayer::new_for_http(),
        // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
        // requests don't hang forever.
        TimeoutLayer::new(Duration::from_secs(10)),
    ));

    // run our app with hyper, and serve it over HTTP
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // Run the server with graceful shutdown
    let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await;
    if let Err(err) = server {
        tracing::error!("Server error: {}", err);
        std::process::exit(1)
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
