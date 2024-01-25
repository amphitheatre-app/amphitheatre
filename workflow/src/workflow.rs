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

use crate::errors::Result;
use crate::{Context, Intent, State};

use kube::runtime::controller::Action;

use std::sync::Arc;

/// Represents the overall workflow orchestrating the execution of states and tasks.
pub struct Workflow<T> {
    pub state: Box<dyn State<T>>,
    pub context: Context<T>,
}

impl<T> Workflow<T> {
    /// Creates a new workflow with an initial state and context.
    pub fn new(context: Context<T>, state: Box<dyn State<T>>) -> Self {
        Workflow { state, context }
    }

    /// Sets the context of the workflow.
    pub fn set_context(&mut self, object: Arc<T>) {
        self.context.object = object;
    }

    /// Transitions to a new state.
    pub fn transition(&mut self, new_state: Box<dyn State<T>>) {
        self.state = new_state;
    }

    /// Runs the workflow until there is no next state to transition to.
    pub async fn run(&mut self) -> Result<Action> {
        while let Some(intent) = self.state.handle(&self.context).await {
            match intent {
                Intent::State(new_state) => {
                    self.transition(new_state);
                }
                Intent::Action(action) => return Ok(action),
            }
        }

        Ok(Action::await_change())
    }
}
