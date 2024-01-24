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

use amp_common::config::Credentials;
use async_nats::jetstream;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents the context shared among different states and tasks.
pub struct Context<T> {
    pub object: Arc<T>,
    pub k8s: Arc<kube::Client>,
    pub credentials: Arc<RwLock<Credentials>>,
    pub jetstream: Arc<jetstream::Context>,
}
