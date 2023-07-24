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

/// The configuration parameters for the application.
///
/// These can either be passed on the command line, or pulled from environment variables.
/// The latter is preferred as environment variables are one of the recommended ways to
/// get configuration from Kubernetes Secrets in deployment.
///
/// This is a pretty simple configuration struct as far as backend APIs go. You could imagine
/// a bunch of other parameters going here, like API keys for external services
/// or flags enabling or disabling certain features or test modes of the API.
///
/// For development convenience, these can also be read from a `.env` file in the working
/// directory where the application is started.
///
/// See `.env.sample` in the repository root for details.
#[derive(Clone, Debug, clap::Parser)]
pub struct Config {
    /// The NATS URL.
    #[clap(long, env = "AMP_NATS_URL")]
    pub nats_url: String,
    /// The workspace path.
    #[clap(long, env = "AMP_WORKSPACE")]
    pub workspace: String,
    /// The playbook identifier.
    #[clap(long, env = "AMP_PLAYBOOK")]
    pub playbook: String,
    /// The actor name.
    #[clap(long, env = "AMP_ACTOR")]
    pub actor: String,
}
