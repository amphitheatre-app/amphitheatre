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

use amp_common::config::Credentials;
use amp_common::docker::{self, registry, DockerConfig};
use amp_common::resource::Actor;
use amp_common::schema::BuildMethod;
use k8s_openapi::api::core::v1::{Container, PodSecurityContext, PodSpec, SecurityContext};
use kube::ResourceExt;
use tracing::{debug, error, info};

use crate::containers::{application, buildpacks, docker_config_volume, git_sync, kaniko, syncer, workspace_volume};
use crate::error::{self, Error, Result};
use crate::{deployment, hash, service};

const DEFAULT_RUN_AS_GROUP: i64 = 1000;
const DEFAULT_RUN_AS_USER: i64 = 1001;

pub struct Deployer {
    k8s: kube::Client,
    credentials: Credentials,
    actor: Actor,
    live: bool,
    once: bool,

    syncer: Option<Container>,
    builder: Option<Container>,
}

impl Deployer {
    pub fn new(k8s: kube::Client, credentials: &Credentials, actor: &Actor) -> Self {
        Self {
            k8s,
            credentials: credentials.clone(),
            actor: actor.clone(),
            live: actor.spec.live,
            once: actor.spec.once,
            syncer: None,
            builder: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        if !self.live && self.built(&self.actor.spec.image).await? {
            info!("The image is already built, skip the build process");
        } else {
            info!("Prepare the build and deploy requirements for building");
            self.prepare().await?;
        }

        // Deploy the actor
        self.deploy().await?;

        // Exposure the actor service
        if self.actor.spec.has_services() {
            self.serve().await?;
        }

        Ok(())
    }

    // Check if the image is already built
    async fn built(&self, image: &str) -> Result<bool> {
        let config = DockerConfig::from(&self.credentials.registries);

        let credential = docker::get_credential(&config, image);
        let credential = match credential {
            Ok(credential) => Some(credential),
            Err(err) => {
                error!("Error handling docker configuration: {}", err);
                None
            }
        };

        if registry::exists(image, credential).await.map_err(Error::DockerRegistryExistsFailed)? {
            info!("The images already exists, Running");
            return Ok(true);
        }

        Ok(false)
    }

    async fn prepare(&mut self) -> Result<()> {
        let build = self.actor.spec.character.build.clone().unwrap_or_default();

        // Get SecurityContext for the container
        let builder = build.buildpacks.clone().unwrap_or_default().builder;
        let security_context = security_context(&builder);

        // Set the syncer
        if self.actor.spec.source.is_some() {
            self.syncer = Some(git_sync::container(&self.actor.spec));
        } else {
            let playbook = owner_reference(&self.actor)?;
            self.syncer = Some(syncer::container(&playbook, &self.actor.spec, &security_context)?);
        }

        // Prefer to use Kaniko to build images with Dockerfile,
        // else, build the image with Cloud Native Buildpacks
        match build.method() {
            BuildMethod::Dockerfile => {
                debug!("Found dockerfile, build it with kaniko");
                self.builder = Some(kaniko::container(&self.actor.spec));
            }
            BuildMethod::Buildpacks => {
                debug!("Build the image with Cloud Native Buildpacks");
                self.builder = Some(buildpacks::container(&self.actor.spec, &security_context));
            }
        };

        Ok(())
    }

    async fn deploy(&self) -> Result<()> {
        let name = self.actor.name_any();
        let namespace = self.actor.namespace().ok_or_else(|| Error::MissingObjectKey(".metadata.namespace"))?;

        let resource = deployment::new(&self.actor, self.pod()?)?;
        debug!("The Deployment resource:\n {:?}\n", resource);

        match deployment::exists(&self.k8s, &namespace, &name).await? {
            true => {
                // Deployment already exists, update it if there are new changes
                info!("Try to refresh an existing Deployment {name}");
                let expected_hash = hash(&self.actor.spec)?;
                deployment::update(&self.k8s, &namespace, &name, resource, expected_hash).await?;
            }
            false => {
                // Create a new Deployment
                deployment::create(&self.k8s, &namespace, resource).await?;
                info!("Created new Deployment: {name}");
            }
        }

        Ok(())
    }

    async fn serve(&self) -> Result<()> {
        let name = self.actor.name_any();
        match service::exists(&self.k8s, &self.actor).await? {
            true => {
                info!("Try to refresh an existing Service {name}");
                service::update(&self.k8s, &self.actor).await?;
            }
            false => {
                service::create(&self.k8s, &self.actor).await?;
                info!("Created new Service: {name}");
            }
        }

        Ok(())
    }

    fn pod(&self) -> Result<PodSpec> {
        let mut pod = PodSpec::default();

        let mut init_containers = vec![];
        let mut containers = vec![];

        // Arrange containers according to synchronization method and frequency
        if self.live {
            // Sync the local source to the server (live), the syncer and builder is required.
            let syncer = self.syncer.as_ref().ok_or_else(|| Error::MissingSyncer)?;
            let builder = self.builder.as_ref().ok_or_else(|| Error::MissingBuilder)?;

            // the syncer in init container if sync once (exit after sync once),
            // else, the syncer is sidecar container, it will keep watching the changes.
            match self.once {
                true => {
                    init_containers.push(syncer.clone());
                    init_containers.push(builder.clone());
                    containers.push(application::container(&self.actor.spec));
                }
                false => {
                    containers.push(syncer.clone());
                    containers.push(builder.clone());
                }
            }
        } else {
            // Pull the source from git repo (not live), and exit after sync once (once).
            // the syncer and builder in init containers, app as the main container.
            if let Some(syncer) = &self.syncer {
                init_containers.push(syncer.clone());
            }

            if let Some(builder) = &self.builder {
                init_containers.push(builder.clone());
            }

            // If the docker image is already built, the syncer and builder will be skipped.
            // The application will be the main container.
            containers.push(application::container(&self.actor.spec));
        }

        pod.init_containers = Some(init_containers);
        pod.containers = containers.clone();
        pod.volumes = Some(vec![workspace_volume(), docker_config_volume()]);

        // Set the security context for the pod
        pod.security_context = Some(PodSecurityContext { fs_group: Some(DEFAULT_RUN_AS_GROUP), ..Default::default() });

        Ok(pod)
    }
}

/// Get the playbook name from the owner reference.
#[inline]
fn owner_reference(actor: &Actor) -> Result<String, error::Error> {
    actor
        .owner_references()
        .iter()
        .find_map(|owner| (owner.kind == "Playbook").then(|| owner.name.clone()))
        .ok_or_else(|| Error::MissingObjectKey(".metadata.ownerReferences"))
}

/// Build SecurityContext for the container by Buildpacks builder mapping.
///
/// |user |group|builder|
/// |-----|-----|-------|
/// |1000 |1000 |heroku*|
/// |1000 |1000 |gcr.io/buildpacks*|
/// |1001 |1000 |paketobuildpacks*|
/// |1001 |1000 |amp-buildpacks*|
/// |1001 |1000 |*|
///
fn security_context(builder: &str) -> Option<SecurityContext> {
    let mut run_as_user = DEFAULT_RUN_AS_USER;

    if builder.starts_with("heroku") || builder.starts_with("gcr.io/buildpacks") {
        run_as_user = 1000;
    }

    Some(SecurityContext {
        run_as_user: Some(run_as_user),
        run_as_group: Some(DEFAULT_RUN_AS_GROUP),
        ..Default::default()
    })
}
