// Copyright 2022 The Amphitheatre Authors.
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
    /// The connection URL for the Postgres database this application should use.
    #[clap(long, env)]
    pub database_url: String,

    /// For more information about Registry Names, Namespaces, Images, Artifacts & Tags,
    /// please visit: https://stevelasker.blog/2020/02/17/registry-namespace-repo-names/
    ///
    /// The registry prefix for its corresponding registry.
    #[clap(long, env)]
    pub registry_url: String,

    /// The path between the unique registry and the repo.
    /// Depending on the registry, it may be nested, or single depth.
    #[clap(long, env)]
    pub registry_namespace: String,

    /// The username of Docker Image Registry
    #[clap(long, env)]
    pub registry_username: String,

    /// The password of Docker Image Registry
    #[clap(long, env)]
    pub registry_password: String,
}
