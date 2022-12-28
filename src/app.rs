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
use kube::Client;
use tower::ServiceBuilder;
use tower_governor::errors::display_error;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;

use crate::config::Config;
use crate::database::{self, Database};
use crate::{routes, swagger};

/// The core type through which handler functions can access common API state.
///
/// This can be accessed by adding a parameter `Extension<Context>` to a handler function's
/// parameters.
///
/// It may not be a bad idea if you need your API to be more modular (turn routes
/// on and off, and disable any unused extension objects) but it's really up to a
/// judgement call.
pub struct Context {
    pub config: Config,
    pub db: Database,
    pub k8s: Client,
}

impl Context {
    pub async fn new(config: Config) -> anyhow::Result<Context> {
        let dsn = config.database_url.clone();
        Ok(Context {
            config,
            db: database::new(dsn).await?,
            k8s: Client::try_default().await?,
        })
    }
}

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
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    display_error(e)
                }))
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