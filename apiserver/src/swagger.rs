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

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{handlers, models};

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::actor::detail,
        handlers::actor::logs,
        handlers::actor::info,
        handlers::actor::stats,
        //
        handlers::playbook::list,
        handlers::playbook::create,
        handlers::playbook::detail,
        handlers::playbook::update,
        handlers::playbook::delete,
        handlers::playbook::start,
        handlers::playbook::stop,
        handlers::playbook::events,
        handlers::actor::list,
    ),
    components(
        schemas(
            models::actor::Actor,
            models::playbook::Playbook,
        )
    ),
    tags(
        (name = "Actors", description = "The Actors Service Handlers"),
        (name = "Playbooks", description = "The Playbooks Service Handlers"),
    ),
)]
struct ApiDoc;

pub fn build() -> SwaggerUi {
    SwaggerUi::new("/swagger").url("/openapi.json", ApiDoc::openapi())
}
