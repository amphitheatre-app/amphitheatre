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

use kube::runtime::events::{Event, EventType, Recorder};
use tracing::{error, info};

pub async fn trace(recorder: &Recorder, message: impl Into<String>) {
    let message: String = message.into();
    info!("{}", message);

    let event = Event {
        type_: EventType::Normal,
        reason: "Tracing".into(),
        note: Some(message),
        action: "Reconciling".into(),
        secondary: None,
    };

    if let Err(err) = recorder.publish(event).await {
        error!("Failed to publish event: {}", err);
    }
}
