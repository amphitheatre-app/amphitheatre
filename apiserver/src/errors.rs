// Copyright (c) The Amphitheatre Authors. All rights reserved.
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

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::error;

#[derive(Serialize, Deserialize, Debug, Error)]
pub enum ApiError {
    #[error("Database Error")]
    DatabaseError,
    #[error("Kubernetes Error: {0}")]
    KubernetesError(String),
    #[error("Internal Server Error")]
    InternalServerError,
    #[error("Not Found")]
    NotFound,
    #[error("Resolve Error")]
    ResolveError,
    #[error("NATS Error: {0}")]
    NatsError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::DatabaseError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::KubernetesError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::InternalServerError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::ResolveError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::NatsError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        error!("{} - {}", status, message);
        (status, Json(json!({ "message": message }))).into_response()
    }
}
