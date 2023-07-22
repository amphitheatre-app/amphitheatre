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

/// Handle an override event.
/// Override workspace's files with payload tarball.
pub fn override_all(workspace: &Path, req: Synchronization) {
    debug!("Received override event, workspace: {:?}, req: {:?}", workspace, req);
}

pub fn create(workspace: &Path, req: Synchronization) {
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
                        std::fs::create_dir_all(parent).unwrap();
                    }
                }

                std::fs::File::create(path).unwrap();
            }
            sync::Path::Directory(path) => {
                let path = workspace.join(path);
                if path.exists() {
                    warn!("Path already exists: {:?}", path);
                    continue;
                }

                std::fs::create_dir_all(path).unwrap();
            }
        }
    }
}

pub fn modify(workspace: &Path, req: Synchronization) {
    debug!("Received modify event, workspace: {:?}, req: {:?}", workspace, req);
}

pub fn rename(_workspace: &Path, _req: Synchronization) {
    error!("Received rename event, nothing to do!")
}

pub fn remove(workspace: &Path, req: Synchronization) {
    debug!("Received remove event, workspace: {:?}, req: {:?}", workspace, req);
    for path in req.paths {
        match path {
            sync::Path::File(file) => {
                let path = workspace.join(file);
                if !path.exists() {
                    warn!("Path does not exist: {:?}", path);
                    continue;
                }

                std::fs::remove_file(path).unwrap();
            }
            sync::Path::Directory(path) => {
                let path = workspace.join(path);
                if !path.exists() {
                    warn!("Path does not exist: {:?}", path);
                    continue;
                }

                trace!("Removing directory: {:?}", path);
                std::fs::remove_dir_all(path).unwrap();
            }
        }
    }
}
