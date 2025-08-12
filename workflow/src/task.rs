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

use crate::{errors::Result, Intent};
use async_trait::async_trait;

use crate::Context;

/// Trait representing a task within a workflow.
#[async_trait]
pub trait Task<T> {
    /// Creates a new instance of the task.
    fn new() -> Self
    where
        Self: Sized; // Explicitly specify the associated type constraint

    /// Checks if the task should be executed based on the provided context.
    fn matches(&self, ctx: &Context<T>) -> bool;

    /// Executes the task using the provided context.
    async fn execute(&self, ctx: &Context<T>) -> Result<Option<Intent<T>>>;
}

#[cfg(test)]
mod tests {
    use crate::{errors::Result, Intent};
    use async_trait::async_trait;

    #[test]
    fn test_impl_task_trait() {
        #[allow(dead_code)]
        struct TestTask {}

        #[async_trait]
        impl super::Task<()> for TestTask {
            fn new() -> Self
            where
                Self: Sized,
            {
                TestTask {}
            }

            fn matches(&self, _ctx: &super::Context<()>) -> bool {
                true
            }

            async fn execute(&self, _ctx: &super::Context<()>) -> Result<Option<Intent<()>>> {
                Ok(None)
            }
        }
    }
}
