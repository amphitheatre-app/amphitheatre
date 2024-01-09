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

use std::path::Path;

use amp_common::sync::{self, Synchronization};
use tar::Archive;
use tracing::{debug, error, info, warn};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Overwrite workspace's files with payload tarball.
pub fn overwrite(workspace: &Path, req: &Synchronization) -> Result<()> {
    debug!("Received overwrite event, workspace: {:?}, req: {:?}", workspace, req);

    if let Some(payload) = &req.payload {
        let mut ar = Archive::new(payload.as_slice());
        ar.unpack(workspace)?;
        info!("Received overwrite event and unpacked into workspace: {:?}", workspace);
    }

    Ok(())
}

/// Create new files or directories.
pub fn create(workspace: &Path, req: &Synchronization) -> Result<()> {
    debug!("Received create event, workspace: {:?}, req: {:?}", workspace, req);
    for path in &req.paths {
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

                std::fs::File::create(path.clone())?;
                info!("Created file: {:?}", path);
            }
            sync::Path::Directory(path) => {
                let path = workspace.join(path);
                if path.exists() {
                    warn!("Path already exists: {:?}", path);
                    continue;
                }

                std::fs::create_dir_all(path.clone())?;
                info!("Created directory: {:?}", path);
            }
        }
    }

    Ok(())
}

/// Modify existing files or directories.
pub fn modify(workspace: &Path, req: &Synchronization) -> Result<()> {
    debug!("Received modify event, workspace: {:?}, req: {:?}", workspace, req);

    if let Some(payload) = &req.payload {
        let mut ar = Archive::new(payload.as_slice());
        ar.unpack(workspace)?;
        info!("Received modify event and unpacked into workspace: {:?}", workspace);
    }

    Ok(())
}

/// Rename existing files or directories.
pub fn rename(_workspace: &Path, _req: &Synchronization) -> Result<()> {
    error!("Received rename event, nothing to do!");
    Ok(())
}

/// Remove existing files or directories.
pub fn remove(workspace: &Path, req: &Synchronization) -> Result<()> {
    debug!("Received remove event, workspace: {:?}, req: {:?}", workspace, req);

    for path in &req.paths {
        match path {
            sync::Path::File(file) => {
                let path = workspace.join(file);
                if !path.exists() {
                    warn!("Path does not exist: {:?}", path);
                    continue;
                }

                std::fs::remove_file(path.clone())?;
                info!("Removed file: {:?}", path);
            }
            sync::Path::Directory(path) => {
                let path = workspace.join(path);
                if !path.exists() {
                    warn!("Path does not exist: {:?}", path);
                    continue;
                }

                std::fs::remove_dir_all(path.clone())?;
                info!("Removed directory: {:?}", path);
            }
        }
    }

    Ok(())
}
