// Copyright 2023 The Amphitheatre Authors.
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

use std::sync::Arc;

use amp_common::sync::Synchronization;
use async_nats::jetstream::{self, stream};
use uuid::Uuid;

use crate::context::Context;
use crate::responses::actor::ActorResponse;
use crate::services::Result;

pub struct ActorService;

impl ActorService {
    pub async fn get(_ctx: Arc<Context>, _pid: Uuid, _name: String) -> Result<ActorResponse> {
        unimplemented!()
    }

    pub async fn list(_ctx: Arc<Context>, _pid: Uuid) -> Result<Vec<ActorResponse>> {
        unimplemented!()
    }

    pub async fn sync(
        ctx: Arc<Context>,
        pid: Uuid,
        name: String,
        req: Synchronization,
    ) -> Result<(), async_nats::Error> {
        // Connect to NATS server and create a JetStream instance
        let client = async_nats::connect(&ctx.config.nats_url).await?;
        let jetstream = jetstream::new(client);

        // Must create a stream before publishing, otherwise the publish will fail.
        jetstream
            .get_or_create_stream(stream::Config {
                name: pid.to_string(),
                subjects: vec![format!("{}.*", pid)],
                ..Default::default()
            })
            .await?;

        // Publish a message to the stream
        let subject = format!("{}.{}", pid, name);
        let payload = serde_json::to_vec(&req)?;
        jetstream.publish(subject, payload.into()).await?.await?;

        Ok(())
    }
}
