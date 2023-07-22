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

use std::path::Path;

use amp_common::sync::{self, Synchronization};
use tracing::{debug, error, trace, warn};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Handle an override event.
/// Override workspace's files with payload tarball.
pub fn replace(workspace: &Path, req: Synchronization) -> Result<()> {
    debug!("Received override event, workspace: {:?}, req: {:?}", workspace, req);
    Ok(())
}

pub fn create(workspace: &Path, req: Synchronization) -> Result<()> {
    debug!("Received create event, workspace: {:?}, req: {:?}", workspace, req);
    for path in req.paths {
        match path {
            sync::Path::File(file) => {
                let path = workspace.join(file);
                if path.exists() {
                    warn!("Path already exists: {:?}", path);
                    continue;
                }

                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)?;
                    }
                }

                std::fs::File::create(path)?;
            }
            sync::Path::Directory(path) => {
                let path = workspace.join(path);
                if path.exists() {
                    warn!("Path already exists: {:?}", path);
                    continue;
                }

                std::fs::create_dir_all(path)?;
            }
        }
    }

    Ok(())
}

pub fn modify(workspace: &Path, req: Synchronization) -> Result<()> {
    debug!("Received modify event, workspace: {:?}, req: {:?}", workspace, req);
    Ok(())
}

pub fn rename(_workspace: &Path, _req: Synchronization) -> Result<()> {
    error!("Received rename event, nothing to do!");
    Ok(())
}

pub fn remove(workspace: &Path, req: Synchronization) -> Result<()> {
    debug!("Received remove event, workspace: {:?}, req: {:?}", workspace, req);
    for path in req.paths {
        match path {
            sync::Path::File(file) => {
                let path = workspace.join(file);
                if !path.exists() {
                    warn!("Path does not exist: {:?}", path);
                    continue;
                }

                std::fs::remove_file(path)?;
            }
            sync::Path::Directory(path) => {
                let path = workspace.join(path);
                if !path.exists() {
                    warn!("Path does not exist: {:?}", path);
                    continue;
                }

                trace!("Removing directory: {:?}", path);
                std::fs::remove_dir_all(path)?;
            }
        }
    }

    Ok(())
}
