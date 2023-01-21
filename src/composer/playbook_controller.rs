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

use kube::api::ListParams;
use kube::error::ErrorResponse;
use kube::runtime::controller::Action;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::{Api, ResourceExt};

use super::Ctx;
use crate::resources::crds::{Actor, ActorSpec, Partner, Playbook, PlaybookState};
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
        let exists: HashSet<String> = self.spec.actors.iter().map(|actor| actor.url()).collect();

        let mut fetches: HashSet<Partner> = HashSet::new();
        for actor in &self.spec.actors {
            if let Some(partners) = &actor.partners {
                for partner in partners {
                    if exists.contains(&partner.url()) {
                        continue;
                    }
                    fetches.insert(partner.clone());
                }
            }
        }

        tracing::debug!("Existing repos are:\n{exists:#?}\nand fetches are: {fetches:#?}");

        for partner in fetches.iter() {
            tracing::info!("fetches url: {}", partner.url());
            let actor = read_partner(partner);
            playbook::add(ctx.client.clone(), self, actor).await?;
        }

        if fetches.is_empty() {
            playbook::patch_status(
                ctx.client.clone(),
                self,
                PlaybookState::running(true, "AutoRun", None),
            )
            .await?;
        }

        Ok(())
    }

    async fn run(&self, ctx: Arc<Ctx>) -> Result<()> {
        for spec in &self.spec.actors {
            match actor::exists(ctx.client.clone(), self, spec).await? {
                true => {
                    // Actor already exists, update it if there are new changes
                    actor::update(ctx.client.clone(), self, spec).await?;
                }
                false => {
                    // Create a new actor
                    actor::create(ctx.client.clone(), self, spec).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn cleanup(&self, ctx: Arc<Ctx>) -> Result<Action> {
        let namespace = self.spec.namespace.clone();
        let api: Api<Actor> = Api::namespaced(ctx.client.clone(), namespace.as_str());

        if let Err(kube::Error::Api(ErrorResponse { reason, .. })) =
            api.list(&ListParams::default().limit(1)).await
        {
            if &reason == "NotFound" {
                tracing::info!("Cleaning up namespace record");
                namespace::delete(ctx.client.clone(), self.spec.namespace.clone()).await?;
            }
        }

        tracing::info!("Waiting for all resources to clean up");
        Ok(Action::await_change())
    }
}

fn read_partner(partner: &Partner) -> ActorSpec {
    ActorSpec {
        name: partner.name.clone(),
        description: "A simple NodeJs example app".into(),
        image: "amp-example-nodejs".into(),
        repository: partner.repository.clone(),
        reference: partner.reference.clone(),
        path: partner.path.clone(),
        commit: "285ef2bc98fb6b3db46a96b6a750fad2d0c566b5".into(),
        ..ActorSpec::default()
    }
}
