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

use std::str::Utf8Error;

use amp_common::http::HTTPError;
use amp_common::scm::errors::SCMError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("ClientError: {0}")]
    ClientError(#[source] HTTPError),

    #[error("InvalidRepoAddress: {0}")]
    InvalidRepoAddress(#[source] url::ParseError),

    #[error("FetchingError: {0}")]
    FetchingError(String),

    #[error("TomlParseFailed: {0}")]
    TomlParseFailed(toml::de::Error),

    #[error("InvalidRegistryAddress: {0}")]
    InvalidRegistryAddress(#[source] url::ParseError),

    #[error("EmptyRegistryAddress")]
    EmptyRegistryAddress,

    #[error("SCMError")]
    SCMError(#[source] SCMError),

    #[error("ResourceError")]
    ResourceError(#[source] amp_resources::error::Error),

    #[error("ConvertBytesError")]
    ConvertBytesError(Utf8Error),

    #[error("UnknownCharacterRegistry")]
    UnknownCharacterRegistry(String),

    #[error("UnknownPreface")]
    UnknownPreface,

    #[error("UnsupportedPartner")]
    UnsupportedPartner,

    #[error("SourceNotSet")]
    SourceNotSet,

    #[error("NameNotSet")]
    NameNotSet,
}

pub type Result<T, E = ResolveError> = std::result::Result<T, E>;
