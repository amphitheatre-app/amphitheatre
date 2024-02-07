// Copyright (c) The Amphitheatre Authors. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;

use axum::response::sse::Event;
use futures::AsyncBufReadExt;
use futures::StreamExt;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::ContainerStatus;
use k8s_openapi::api::core::v1::Pod;
use kube::api::LogParams;
use kube::runtime::watcher::Config;
use kube::runtime::{watcher, WatchStreamExt};
use kube::Api;
use kube::ResourceExt;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct Logger {
    api: Api<Pod>,                            // The Kubernetes API client.
    sender: Sender<Event>,                    // The sender of the log stream.
    config: Config,                           // The configuration of watcher.
    watches: HashMap<String, JoinHandle<()>>, // The map of watching containers.
}

impl Logger {
    /// Creates a new logger.
    pub fn new(client: kube::Client, sender: Sender<Event>, playbook: Uuid, actor: String) -> Self {
        let api: Api<Pod> = Api::namespaced(client, &format!("amp-{playbook}"));
        let label_selector = format!("amphitheatre.app/character={actor}");
        let config = Config::default().labels(&label_selector);

        Self { api, sender, config, watches: HashMap::new() }
    }

    /// Starts the logger.
    pub async fn start(&mut self) {
        let watcher = watcher(self.api.clone(), self.config.clone());
        let mut watcher = watcher.touched_objects().boxed();

        while let Some(pod) = watcher.try_next().await.unwrap() {
            let pod_name = pod.name_any();
            if let Some(status) = pod.status {
                // Unsubscribe all the watches of the pod if pod is terminating.
                // and then break the loop to exit the function.
                if status.phase == Some("Terminating".into()) {
                    self.unsubscribe_all(&pod_name);
                    return;
                }

                self.watches(&pod_name, status.init_container_statuses).await;
                self.watches(&pod_name, status.container_statuses).await;
            }
        }
    }

    /// Watches the containers of the pod.
    async fn watches(&mut self, pod: &str, containers: Option<Vec<ContainerStatus>>) {
        if containers.is_none() {
            warn!("No container statuses found in pod {}.", pod);
            return;
        }

        // Iterate the containers of the pod.
        for container in containers.unwrap() {
            if container.state.is_none() {
                warn!("No state found in container {} of {}.", container.name, pod);
                continue;
            }
            let state = container.state.unwrap();

            // If the container is running, then subscribe the log stream.
            if state.running.is_some_and(|s| s.started_at.is_some()) {
                let key = &format!("{}-{}", pod, container.name);
                if self.watches.contains_key(key) {
                    debug!("Skip container {} of {} because it's watching.", &container.name, pod);
                    continue;
                }
                self.subscribe(pod, &container.name).await;
            }

            // If the container is terminated, then unsubscribe the log stream.
            if state.terminated.is_some_and(|s| s.finished_at.is_some()) {
                debug!("Container {} in {} has been terminated.", container.name, pod);
                self.unsubscribe(pod, &container.name);
            }
        }
    }

    /// Subscribes the log stream of the container.
    async fn subscribe(&mut self, pod: &str, container: &str) {
        let key = format!("{}-{}", pod, container);

        let api = self.api.clone();
        let sender = self.sender.clone();
        let container = container.to_string();
        let pod = pod.to_string();

        let task = tokio::spawn(async move {
            Self::tail(api, sender, pod, container).await;
        });

        self.watches.insert(key, task);
    }

    /// Tails the log stream of the container.
    async fn tail(api: Api<Pod>, sender: Sender<Event>, pod: String, container: String) {
        let params = LogParams {
            container: Some(container.to_string()),
            follow: true,
            tail_lines: Some(100),
            timestamps: false,
            ..Default::default()
        };

        match api.log_stream(&pod, &params).await {
            Ok(stream) => {
                info!("Start to receive the log stream of container {} in {}...", container, pod);
                let mut lines = stream.lines();
                while let Ok(Some(line)) = lines.try_next().await {
                    _ = sender.send(Event::default().data(line)).await;
                }
            }
            Err(err) => {
                let message =
                    format!("Some error occurred while log stream for container {} in {}: {}.", container, pod, err);
                error!("{}", message);
                _ = sender.send(Event::default().data(message)).await;
            }
        }
    }

    /// Unsubscribes all the log streams.
    fn unsubscribe_all(&mut self, pod: &str) {
        for (key, task) in self.watches.drain() {
            if key.starts_with(pod) {
                info!("Unsubscribe the log stream of container {} in {}.", key, pod);
                task.abort();
            }
        }
    }

    /// Unsubscribes the log stream of the container.
    fn unsubscribe(&mut self, pod: &str, container: &str) {
        let key = &format!("{}-{}", pod, container);
        if let Some(task) = self.watches.remove(key) {
            info!("Unsubscribe the log stream of container {} in {}.", container, pod);
            task.abort();
        }
    }
}
