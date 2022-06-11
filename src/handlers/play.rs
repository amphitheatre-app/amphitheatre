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

use rocket::serde::json::Json;
use crate::models::play::Play;

#[get("/")]
pub fn list() -> Json<Vec<Play>>{
    Json(vec![Play::default()])
}

#[get("/<id>")]
pub fn detail(id: u64) -> Json<Play> {
    Json(Play::default())
}