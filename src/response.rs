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

pub type Result<T, E = ApiError> = axum::response::Result<Response<T>, E>;

/// Represents the response from an API call
#[derive(Serialize, Deserialize, Debug)]
pub enum Response<T> {
    EmptyResponse,
    SingleResponse {
        data: T,
    },
    PagedResponse {
        /// The object or a Vec<T> objects (the type `T` will depend on the endpoint).
        data: T,
        /// Any API endpoint that returns a list of items requires pagination.
        pagination: Pagination,
    },
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

#[derive(Serialize, Deserialize, Debug)]
pub enum ApiError {
    NotFound,
    InternalServerError,
}

impl<T: serde::Serialize> IntoResponse for Response<T> {
    fn into_response(self) -> axum::response::Response {
        let body = match self {
            Response::EmptyResponse => json!({ "data": "" }),
            Response::SingleResponse { data } => json!({ "data": data }),
            Response::PagedResponse { data, pagination } => {
                json!({ "data": data, "pagination": pagination })
            }
        };

        Json(body).into_response()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let response = match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({ "message": "Not Found"})),
            ),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message": "Internal Server Error"})),
            ),
        };
        response.into_response()
    }
}

/// Return the successful response without data.
pub fn empty<T>(_: T) -> Result<T> {
    Ok(Response::EmptyResponse)
}

/// Returns the successful response with single data.
pub fn success<T>(data: T) -> Result<T> {
    Ok(Response::SingleResponse { data })
}

/// Returns the successful paged response.
pub fn paginate<T>(data: T, pagination: Pagination) -> Result<T> {
    Ok(Response::PagedResponse { data, pagination })
}
