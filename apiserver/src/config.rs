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
#[derive(clap::Parser)]
pub struct Config {
    /// The Database server host.
    #[clap(long, env = "AMP_DATABASE_HOST")]
    pub database_host: String,
    /// The Database server port.
    #[clap(long, env = "AMP_DATABASE_PORT")]
    pub database_port: String,
    /// The Database name.
    #[clap(long, env = "AMP_DATABASE_NAME")]
    pub database_name: String,
    /// The Database username.
    #[clap(long, env = "AMP_DATABASE_USERNAME")]
    pub database_username: String,
    /// The Database password.
    #[clap(long, env = "AMP_DATABASE_PASSWORD")]
    pub database_password: String,

    /// The Server port.
    #[clap(long, env = "AMP_PORT")]
    pub port: u16,
}
