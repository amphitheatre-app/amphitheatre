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

use async_trait::async_trait;

use crate::Context;

/// Trait representing the state of a workflow.
#[async_trait]
pub trait State<T>: Send + Sync {
    /// Handles the current state and may transition to a new state.
    async fn handle(&self, ctx: &Context<T>) -> Option<Box<dyn State<T>>>;
}

#[cfg(test)]
mod tests {
    use super::Context;
    use super::State;
    use async_trait::async_trait;

    #[test]
    fn test_impl_state_trait() {
        struct TestState {}

        #[async_trait]
        impl State<()> for TestState {
            async fn handle(&self, _ctx: &Context<()>) -> Option<Box<dyn State<()>>> {
                None
            }
        }
    }
}
