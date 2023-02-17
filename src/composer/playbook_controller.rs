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

use k8s_openapi::api::core::v1::ObjectReference;
use kube::runtime::controller::Action;
use kube::runtime::events::Recorder;
use kube::runtime::finalizer::{finalizer, Event as FinalizerEvent};
use kube::{Api, Resource, ResourceExt};
use url::Url;

use crate::context::Context;
use crate::resources::crds::{Partner, Playbook, PlaybookState};
use crate::resources::error::{Error, Result};
use crate::resources::event::trace;
use crate::resources::secret::{self, Credential, Kind};
use crate::resources::{actor, namespace, playbook, service_account};
use crate::services::actor::ActorService;

/// The reconciler that will be called when either object change
pub async fn reconcile(playbook: Arc<Playbook>, ctx: Arc<Context>) -> Result<Action> {
    tracing::info!("Reconciling Playbook \"{}\"", playbook.name_any());
    if playbook.spec.actors.is_empty() {
        return Err(Error::EmptyActorsError);
    }

    let api: Api<Playbook> = Api::all(ctx.k8s.clone());
    let recorder = ctx.recorder(playbook.reference());

    // Reconcile the playbook custom resource.
    let finalizer_name = "playbooks.amphitheatre.app/finalizer";
    finalizer(&api, finalizer_name, playbook, |event| async {
        match event {
            FinalizerEvent::Apply(playbook) => playbook.reconcile(ctx.clone(), &recorder).await,
            FinalizerEvent::Cleanup(playbook) => playbook.cleanup(ctx.clone(), &recorder).await,
        }
    })
    .await
    .map_err(|e| Error::FinalizerError(Box::new(e)))
}
/// an error handler that will be called when the reconciler fails with access to both the
/// object that caused the failure and the actual error
pub fn error_policy(_playbook: Arc<Playbook>, error: &Error, _ctx: Arc<Context>) -> Action {
    tracing::error!("reconcile failed: {:?}", error);
    Action::requeue(Duration::from_secs(60))
}

impl Playbook {
    pub async fn reconcile(&self, ctx: Arc<Context>, recorder: &Recorder) -> Result<Action> {
        if let Some(ref status) = self.status {
            if status.pending() {
                self.init(ctx, recorder).await?
            } else if status.solving() {
                self.solve(ctx, recorder).await?
            } else if status.running() {
                self.run(ctx, recorder).await?
            }
        }

        Ok(Action::await_change())
        // If no events were received, check back every 2 minutes
        // Ok(Action::requeue(Duration::from_secs(2 * 60)))
    }

    /// Init create namespace, credentials and service accounts
    async fn init(&self, ctx: Arc<Context>, recorder: &Recorder) -> Result<()> {
        let namespace = &self.spec.namespace;

        // Create namespace for this playbook
        namespace::create(ctx.k8s.clone(), self).await?;
        trace(recorder, "Created namespace for this playbook").await?;

        // Docker registry Credential
        let credential = Credential::basic(
            Kind::Image,
            Url::parse(&ctx.config.registry_url).map_err(Error::UrlParseError)?,
            ctx.config.registry_username.clone(),
            ctx.config.registry_password.clone(),
        );

        trace(recorder, "Creating Secret for Docker Registry Credential").await?;
        secret::create(ctx.k8s.clone(), namespace.clone(), &credential).await?;

        // Patch this credential to default service account
        trace(recorder, "Patch the credential to default service account").await?;
        service_account::patch(ctx.k8s.clone(), namespace, "default", &credential, true, true).await?;

        trace(recorder, "Init successfully, Let's begin solve, now!").await?;
        playbook::patch_status(ctx.k8s.clone(), self, PlaybookState::solving()).await?;

        Ok(())
    }

    async fn solve(&self, ctx: Arc<Context>, recorder: &Recorder) -> Result<()> {
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
            let actor = ActorService::read(&ctx, partner)
                .await
                .map_err(Error::ApiError)?
                .unwrap();

            trace(recorder, "Fetch and add the actor to this playbook").await?;
            playbook::add(ctx.k8s.clone(), self, actor).await?;
        }

        if fetches.is_empty() {
            trace(recorder, "Solved successfully, Running").await?;
            playbook::patch_status(
                ctx.k8s.clone(),
                self,
                PlaybookState::running(true, "AutoRun", None),
            )
            .await?;
        }

        Ok(())
    }

    async fn run(&self, ctx: Arc<Context>, recorder: &Recorder) -> Result<()> {
        for spec in &self.spec.actors {
            match actor::exists(ctx.k8s.clone(), self, spec).await? {
                true => {
                    // Actor already exists, update it if there are new changes
                    trace(
                        recorder,
                        format!(
                            "Actor {} already exists, update it if there are new changes",
                            spec.name
                        ),
                    )
                    .await?;
                    actor::update(ctx.k8s.clone(), self, spec).await?;
                }
                false => {
                    // Create a new actor
                    trace(recorder, format!("Create new Actor: {}", spec.name)).await?;
                    actor::create(ctx.k8s.clone(), self, spec).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn cleanup(&self, _ctx: Arc<Context>, _recorder: &Recorder) -> Result<Action> {
        Ok(Action::await_change())
    }

    fn reference(&self) -> ObjectReference {
        let mut reference = self.object_ref(&());
        reference.namespace = Some(self.spec.namespace.to_string());
        reference
    }
}
