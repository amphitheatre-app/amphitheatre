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

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use kube::runtime::controller::Action;
use kube::runtime::events::{Event, EventType};
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::{Api, Resource, ResourceExt};

use super::Ctx;
use crate::resources::crds::{ActorSpec, Playbook, PlaybookState};
use crate::resources::error::{Error, Result};
use crate::resources::secret::{self, Credential, Kind};
use crate::resources::{actor, namespace, playbook, service_account};

/// The reconciler that will be called when either object change
pub async fn reconcile(playbook: Arc<Playbook>, ctx: Arc<Ctx>) -> Result<Action> {
    let api: Api<Playbook> = Api::all(ctx.client.clone());

    tracing::info!("Reconciling Playbook \"{}\"", playbook.name_any());
    if playbook.spec.actors.is_empty() {
        return Err(Error::EmptyActorsError);
    }

    let finalizer_name = "playbooks.amphitheatre.app/finalizer";
    finalizer(&api, finalizer_name, playbook, |event| async {
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
        if let Some(ref status) = self.status {
            if status.pending() {
                self.init(ctx).await?
            } else if status.solving() {
                self.solve(ctx).await?
            } else if status.running() {
                self.run(ctx).await?
            }
        } else {
            tracing::debug!("Waiting for PlaybookStatus to be reported, not starting yet");
        }

        Ok(Action::await_change())
    }

    /// Init create namespace, credentials and service accounts
    async fn init(&self, ctx: Arc<Ctx>) -> Result<()> {
        let namespace = &self.spec.namespace;

        // Create namespace for this playbook
        namespace::create(ctx.client.clone(), namespace).await?;

        // Docker registry Credential
        let credential = Credential::basic(
            Kind::Image,
            "harbor.amp-system.svc.cluster.local".into(),
            "admin".into(),
            "Harbor12345".into(),
        );

        secret::create(ctx.client.clone(), namespace.clone(), &credential).await?;

        // Patch this credential to default service account
        service_account::patch(
            ctx.client.clone(),
            namespace,
            "default",
            &credential,
            true,
            true,
        )
        .await?;

        playbook::patch_status(ctx.client.clone(), self, PlaybookState::solving()).await?;

        Ok(())
    }

    async fn solve(&self, ctx: Arc<Ctx>) -> Result<()> {
        let exists: HashSet<String> = self.spec.actors.iter().map(|a| a.url()).collect();
        let mut fetches: HashSet<String> = HashSet::new();

        for actor in &self.spec.actors {
            if let Some(partners) = &actor.partners {
                for partner in partners {
                    let url = partner.url();
                    if exists.contains(&url) {
                        continue;
                    }
                    fetches.insert(url);
                }
            }
        }

        for url in fetches.iter() {
            tracing::info!("fetches url: {}", url);
            let actor = read_partner(url);
            actor::add(ctx.client.clone(), self, actor).await?;
        }

        tracing::info!("fetches length: {}", fetches.len());

        if fetches.is_empty() {
            playbook::patch_status(ctx.client.clone(), self, PlaybookState::ready()).await?;
        }

        Ok(())
    }

    async fn run(&self, ctx: Arc<Ctx>) -> Result<()> {
        for actor in &self.spec.actors {
            actor::build(ctx.client.clone(), self, actor).await?;
            actor::deploy(ctx.client.clone(), self, actor).await?;
        }
        Ok(())
    }

    pub async fn cleanup(&self, ctx: Arc<Ctx>) -> Result<Action> {
        // todo add some deletion event logging, db clean up, etc.?
        let recorder = ctx.recorder(self.object_ref(&()));
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

fn read_partner(url: &String) -> ActorSpec {
    ActorSpec {
        name: "amp-example-nodejs".into(),
        description: "A simple NodeJs example app".into(),
        image: "amp-example-nodejs".into(),
        repository: url.into(),
        reference: "master".into(),
        commit: "285ef2bc98fb6b3db46a96b6a750fad2d0c566b5".into(),
        ..ActorSpec::default()
    }
}
