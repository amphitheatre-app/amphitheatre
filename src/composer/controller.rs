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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use kube::runtime::controller::Action;
use kube::runtime::events::{Event, EventType, Recorder};
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::{Api, Client, Resource, ResourceExt};

use super::error::{Error, Result};
use super::resource;
use super::types::{Actor, Playbook, PlaybookStatus, PLAYBOOK_RESOURCE_NAME};

pub struct Ctx {
    /// Kubernetes client
    pub client: Client,
}

impl Ctx {
    fn recorder(&self, playbook: &Playbook) -> Recorder {
        Recorder::new(
            self.client.clone(),
            "amphitheatre-composer".into(),
            playbook.object_ref(&()),
        )
    }
}

/// The reconciler that will be called when either object change
pub async fn reconcile(playbook: Arc<Playbook>, ctx: Arc<Ctx>) -> Result<Action> {
    let ns = playbook.namespace().unwrap(); // doc is namespace scoped
    let api: Api<Playbook> = Api::namespaced(ctx.client.clone(), &ns);

    tracing::info!("Reconciling Playbook \"{}\" in {}", playbook.name_any(), ns);
    if playbook.spec.actors.is_empty() {
        return Err(Error::EmptyActorsError);
    }

    finalizer(&api, PLAYBOOK_RESOURCE_NAME, playbook, |event| async {
        match event {
            FinalizerEvent::Apply(playbook) => playbook.reconcile(ctx.clone()).await,
            FinalizerEvent::Cleanup(playbook) => playbook.cleanup(ctx.clone()).await,
        }
    })
    .await
    .map_err(|e| Error::FinalizerError(Box::new(e)))
}
/// an error handler that will be called when the reconciler fails with access to both the
/// object that caused the failure and the actual error
pub fn error_policy(playbook: Arc<Playbook>, error: &Error, ctx: Arc<Ctx>) -> Action {
    tracing::error!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(60))
}

impl Playbook {
    pub async fn reconcile(&self, ctx: Arc<Ctx>) -> Result<Action> {
        let recorder = ctx.recorder(self);

        let status = self.status.clone().unwrap_or(PlaybookStatus::Pending);
        match status {
            PlaybookStatus::Pending => {
                resource::status(ctx.client.clone(), self, PlaybookStatus::Solving).await?;
                recorder
                    .publish(Event {
                        type_: EventType::Normal,
                        reason: "SolvingPlaybook".into(),
                        note: Some(format!("Solving playbook `{}`", self.name_any())),
                        action: "Reconciling".into(),
                        secondary: None,
                    })
                    .await
                    .map_err(Error::KubeError)?;
            }
            PlaybookStatus::Solving => {
                let exists: HashSet<String> =
                    self.spec.actors.iter().map(|a| a.repo.clone()).collect();
                let mut fetches: HashSet<String> = HashSet::new();

                for actor in &self.spec.actors {
                    if actor.partners.is_empty() {
                        continue;
                    }

                    for repo in &actor.partners {
                        if exists.contains(repo) {
                            continue;
                        }
                        fetches.insert(repo.to_string());
                    }
                }

                for url in fetches.iter() {
                    tracing::info!("fetches url: {}", url);
                    recorder
                        .publish(Event {
                            type_: EventType::Normal,
                            reason: "ReadPartner".into(),
                            note: Some(format!(
                                "Reading partner from `{} for {}`",
                                url,
                                self.name_any()
                            )),
                            action: "Reconciling".into(),
                            secondary: None,
                        })
                        .await
                        .map_err(Error::KubeError)?;

                    let actor: Actor = read_partner(url);
                    resource::add(ctx.client.clone(), self, actor).await?;
                }

                tracing::info!("fetches length: {}", fetches.len());

                if fetches.is_empty() {
                    resource::status(ctx.client.clone(), self, PlaybookStatus::Solved).await?;
                    recorder
                        .publish(Event {
                            type_: EventType::Normal,
                            reason: "SolvedPlaybook".into(),
                            note: Some(format!("Solved playbook `{}`", self.name_any())),
                            action: "Reconciling".into(),
                            secondary: None,
                        })
                        .await
                        .map_err(Error::KubeError)?;
                }
            }
            PlaybookStatus::Solved => {
                for actor in &self.spec.actors {
                    resource::build(ctx.client.clone(), self, actor).await?;
                }
            }
            PlaybookStatus::Building => todo!(),
            PlaybookStatus::Running => todo!(),
            PlaybookStatus::Succeeded => todo!(),
            PlaybookStatus::Failed => todo!(),
            PlaybookStatus::Unknown => todo!(),
        };

        // If no events were received, check back every 5 minutes
        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    pub async fn cleanup(&self, ctx: Arc<Ctx>) -> Result<Action> {
        // todo add some deletion event logging, db clean up, etc.?
        let recorder = ctx.recorder(self);
        // Doesn't have dependencies in this example case, so we just publish an event
        recorder
            .publish(Event {
                type_: EventType::Normal,
                reason: "DeletePlaybook".into(),
                note: Some(format!("Delete playbook `{}`", self.name_any())),
                action: "Reconciling".into(),
                secondary: None,
            })
            .await
            .map_err(Error::KubeError)?;
        Ok(Action::await_change())
    }
}

fn read_partner(url: &String) -> Actor {
    Actor {
        name: "amp-example-rust-demo".into(),
        description: "A simple Rust example app".into(),
        image: "amp-example-rust-demo".into(),
        repo: url.into(),
        path: ".".into(),
        reference: "master".into(),
        commit: "d582e8ddf81177ecf2ae6b136642868ba089a898".into(),
        environment: HashMap::new(),
        partners: vec![],
    }
}
