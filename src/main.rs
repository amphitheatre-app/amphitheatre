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

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_sync_db_pools;

use kube::Client;

mod database;
mod handlers;
mod models;
mod repositories;
mod routes;
mod schema;
mod services;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let client = Client::try_default().await.unwrap();

    let _= routes::build()
        .attach(database::stage())
        .manage(client)
        .launch().await?;

    Ok(())
}
