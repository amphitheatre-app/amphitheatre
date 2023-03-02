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

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

/// Represents the response from an API call
#[derive(Serialize, Deserialize, Debug)]
pub struct Response<T> {
    /// The object or a Vec<T> objects (the type `T` will depend on the endpoint).
    data: Option<T>,
    /// Any API endpoint that returns a list of items requires pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pagination: Option<Pagination>,
}

/// Any API endpoint that returns a list of items requires pagination.
/// By default we will return 30 records from any listing endpoint. If an API
/// endpoint returns a list of items, then it will include a pagination object
/// that contains pagination information.
#[derive(Serialize, Deserialize, Debug)]
pub struct Pagination {
    /// The page currently returned (default: 1)
    pub current_page: u64,
    /// The number of entries returned per page (default: 30)
    pub per_page: u64,
    /// The Total number of entries available in the entries collection.
    pub total_entries: u64,
    /// The total number of pages available given the current `per_page` value
    pub total_pages: u64,
}

impl<T: Serialize> IntoResponse for Response<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

/// Returns the successful response with data.
pub fn data<T>(data: T) -> Response<T> {
    Response {
        data: Some(data),
        pagination: None,
    }
}

/// Returns the successful paged response.
pub fn paginate<T>(data: T, pagination: Pagination) -> Response<T> {
    Response {
        data: Some(data),
        pagination: Some(pagination),
    }
}

#[derive(Serialize, Deserialize, Debug, Error)]
pub enum ApiError {
    #[error("Database Error")]
    DatabaseError,
    #[error("Kubernetes Error")]
    KubernetesError,
    #[error("Internal Server Error")]
    InternalServerError,
    #[error("Not Found")]
    NotFound,
    #[error("Resolve Error")]
    ResolveError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::DatabaseError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::KubernetesError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::InternalServerError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::ResolveError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, Json(json!({ "message": message }))).into_response()
    }
}
