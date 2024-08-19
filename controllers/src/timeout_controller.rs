use std::sync::Arc;

use amp_common::resource::Playbook;
use amp_resources::playbook::delete;
use chrono::{DateTime, Duration, TimeDelta, Utc};
use futures::{StreamExt, TryStreamExt};
use kube::runtime::controller::Action;
use kube::Client;
use kube::{
    api::DeleteParams,
    runtime::{reflector, watcher, WatchStreamExt},
    Api,
};
use tracing::{error, info, warn};

use crate::context::Context;

enum Strategy {
    Expired,
    Remain(TimeDelta)
}

impl From<DateTime<Utc>> for Strategy {
    fn from(value: DateTime<Utc>) -> Self {
        let now = Utc::now();
        if value < now {
            Self::Expired
        } else {
            Self::Remain(value - now)
        }
    }
}

pub async fn new(ctx: &Arc<Context>) {
    let client = ctx.k8s.clone();
    
    let api = Api::<Playbook>::all(client.clone());

    let config = watcher::Config::default();

    let (reader, writer) = reflector::store();

    let _rf = reflector(writer, watcher(api, config));

    tokio::spawn(async move {
        reader.wait_until_ready().await.unwrap();
        loop {
            for p in reader.state() {
                if let Err(err) = handle3(p.as_ref(), &client).await {
                    error!("Delete playbook failed: {}", err.to_string());
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
        }
    });

    // let mut obs = watcher(api.clone(), config).reflect(writer).applied_objects().boxed();

    // loop {
    //     let playbook = obs.try_next().await;
    //     match playbook {
    //         Ok(Some(playbook)) => {
    //             if let Err(err) = handle(&playbook, &api).await {
    //                 error!("Delete playbook failed: {}", err.to_string());
    //             }
    //         }
    //         Ok(None) => continue,
    //         Err(err) => {
    //             error!("Resolve playbook stream failed: {}", err.to_string());
    //             continue;
    //         }
    //     }
    //     tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
    // }
}

// async fn handle(playbook: &Playbook, api: &Api<Playbook>) -> anyhow::Result<()> {
//     if let Some(annotations) = &playbook.metadata.annotations {
//         if let Some(ttl_str) = annotations.get("ttl") {
//             if let Ok(ttl) = ttl_str.parse::<i64>() {
//                 if let Some(creation_time) = &playbook.metadata.creation_timestamp {
//                     let now = Utc::now();
//                     let completion_time = creation_time.0;
//                     let expiration_time = completion_time + Duration::seconds(ttl);
//                     if expiration_time - now <= Duration::days(3) {
//                         send_message().await;
//                     } else if expiration_time < now {
//                         if let Some(name) = &playbook.metadata.name {
//                             api.delete(&name, &DeleteParams::default()).await?;
//                         } else {
//                             info!("Playbook name is missing");
//                         }
//                     }
//                 } else {
//                     info!("creation_time not found");
//                 }
//             } else {
//                 info!("Failed to parse TTL value");
//             }
//         } else {
//             info!("TTL annotation not found");
//         }
//     } else {
//         info!("No annotations found");
//     }
//     Ok(())
// }

async fn handle2(playbook: &Playbook, client: &Client) -> anyhow::Result<()> {
    if let Some(expiration_time) = playbook
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get("ttl"))
        .and_then(|ttl_str| ttl_str.parse::<i64>().ok())
        .and_then(|ttl| {
            playbook.metadata.creation_timestamp.as_ref().map(|creation_time| creation_time.0 + Duration::seconds(ttl))
        })
    {
        let now = Utc::now();
        if expiration_time - now <= Duration::days(3) {
            send_message().await;
        } else if expiration_time < now {
            if let Some(name) = &playbook.metadata.name {
                delete(client, name).await?;
            }
        }
    }
    Ok(())
}

async fn handle3(playbook: &Playbook, client: &Client) -> anyhow::Result<()> {
    if let Some(expiration_time) = playbook
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get("ttl"))
        .and_then(|ttl_str| ttl_str.parse::<i64>().ok())
        .and_then(|ttl| {
            playbook.metadata.creation_timestamp.as_ref().map(|creation_time| creation_time.0 + Duration::seconds(ttl))
        })
    {
        let stratege = Strategy::from(expiration_time);
        match stratege {
            Strategy::Expired => {
                if let Some(name) = &playbook.metadata.name {
                    delete(client, name).await?;
                }
            },
            Strategy::Remain(time) => {
                if time <= Duration::days(3)  {
                    send_message().await;
                }
            },
        }
    }
    Ok(())
}

async fn send_message() {
    warn!("Email sending functionality is not implemented yet");
}
