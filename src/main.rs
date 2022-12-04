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

#![allow(unused_variables)]

use std::net::SocketAddr;

use amphitheatre::database::Database;
use amphitheatre::routes;
use axum::error_handling::HandleErrorLayer;
use axum::http::StatusCode;
use axum::{BoxError, Extension};
use tower::ServiceBuilder;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;

#[tokio::main]
async fn main() {
    let database = Database::new();

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(1024)
        .burst_size(1024)
        .use_headers()
        .finish()
        .unwrap();
    // build our application with a single route
    let app = routes::build().layer(Extension(database)).layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|err: BoxError| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled error: {}", err),
                )
            }))
            .layer(GovernorLayer {
                config: &governor_conf,
            }),
    );

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
