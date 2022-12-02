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

use rocket::{Build, Rocket};

use super::handlers::*;

pub fn build() -> Rocket<Build> {
    rocket::build()
        .mount(
            "/v1/actors",
            routes![
                actor::detail, // GET    /v1/actors/<id>
                actor::logs,   // GET    /v1/actors/<id>/logs
                actor::info,   // GET    /v1/actors/<id>/info
                actor::stats,  // GET    /v1/actors/<id>/stats
            ],
        )
        .mount(
            "/v1/playbooks",
            routes![
                playbook::list,   // GET    /v1/playbooks
                playbook::create, // POST   /v1/playbooks
                playbook::detail, // GET    /v1/playbooks/<id>
                playbook::update, // PATCH  /v1/playbooks/<id>
                playbook::delete, // DELETE /v1/playbooks/<id>
                playbook::events, // GET    /v1/playbooks/<id>/events
                playbook::start,  // POST   /v1/playbooks/<id>/actions/start
                playbook::stop,   // POST   /v1/playbooks/<id>/actions/stop
                actor::list,      // GET    /v1/playbooks/<pid>/actors
            ],
        )
}
