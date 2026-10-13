use futures_util::{Stream, StreamExt};
use operator::{
    kube::{
        runtime::{
            watcher::{self, Config, Event},
            WatchStreamExt,
        },
        Api, Client, ResourceExt,
    },
    OgmiosPort,
};
use std::{collections::HashMap, sync::Arc};
use tokio::pin;
use tracing::{error, info, instrument, warn};

use crate::{Consumer, State};

#[instrument("auth background service", skip_all)]
pub fn start(state: Arc<State>) {
    tokio::spawn(async move {
        let client = Client::try_default()
            .await
            .expect("failed to create kube client");

        let api = Api::<OgmiosPort>::all(client.clone());
        // The backoff never gives up, so the stream only ever yields.
        let stream = watcher::watcher(api.clone(), Config::default()).default_backoff();

        run(state, stream).await;

        error!("auth: CRD watch stream ended.");
        std::process::exit(1);
    });
}

/// Apply watcher events to the consumer map until the stream ends.
///
/// A watch error is logged and polling continues: the watcher recovers on its
/// next poll, re-listing through `Restarted` when the error was the routine
/// `410` expiry of its resource version. The API server expires every watcher
/// at once, so exiting here dropped every WebSocket on every replica at once.
pub(crate) async fn run<S>(state: Arc<State>, stream: S)
where
    S: Stream<Item = Result<Event<OgmiosPort>, watcher::Error>>,
{
    pin!(stream);

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => apply(&state, event).await,
            // Unexpected error when streaming CRDs.
            Err(err) => {
                warn!(
                    error = err.to_string(),
                    "auth: Failed to update crds, retrying."
                );
            }
        }
    }
}

/// Ports are created without status, and only a port with status has a key to
/// authenticate. A status update fires a new `Applied` event.
fn consumer(crd: &OgmiosPort) -> Option<Consumer> {
    crd.status.as_ref().map(|_| Consumer::from(crd))
}

// Connection counts are not part of the consumer map (see `Connections`), so
// replacing an entry here never touches how many connections a port has open.
async fn apply(state: &State, event: Event<OgmiosPort>) {
    match event {
        // Stream restart, also run on startup.
        Event::Restarted(crds) => {
            info!("auth: Watcher restarted, reseting consumers");
            let consumers: HashMap<String, Consumer> = crds
                .iter()
                .filter_map(consumer)
                .map(|consumer| (consumer.key.clone(), consumer))
                .collect();
            *state.consumers.write().await = consumers;

            // When the watcher is restarted, we reset the limiter because a user
            // could have changed the tier on the watcher restart.
            state.limiter.write().await.clear();
        }
        // New port created or updated.
        Event::Applied(crd) => match consumer(&crd) {
            Some(consumer) => {
                info!("auth: Adding new consumer: {}", crd.name_any());
                state.limiter.write().await.remove(&consumer.key);
                state
                    .consumers
                    .write()
                    .await
                    .insert(consumer.key.clone(), consumer);
            }
            None => {
                info!("auth: New port created: {}", crd.name_any());
            }
        },
        // Port deleted.
        Event::Deleted(crd) => {
            info!(
                "auth: Port deleted, removing from state: {}",
                crd.name_any()
            );
            if let Some(consumer) = consumer(&crd) {
                state.consumers.write().await.remove(&consumer.key);
                state.limiter.write().await.remove(&consumer.key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::connections::ConnectionLabels;
    use futures_util::stream;
    use operator::{kube::core::ErrorResponse, OgmiosPortSpec, OgmiosPortStatus};
    use std::time::Duration;

    /// The internal tier's cap is 450; one keeps the arithmetic visible.
    const MAX_CONNECTIONS: usize = 1;

    fn state() -> Arc<State> {
        let config = Config {
            proxy_addr: "0.0.0.0:9443".into(),
            proxy_namespace: "ftr-ogmios-v1".into(),
            proxy_tiers_path: "tiers.toml".into(),
            proxy_tiers_poll_interval: Duration::from_secs(2),
            prometheus_addr: "0.0.0.0:9187".into(),
            ogmios_port: 1337,
            ogmios_dns: "ftr-ogmios-v1.svc.cluster.local".into(),
            ssl_crt_path: "localhost.crt".into(),
            ssl_key_path: "localhost.key".into(),
            health_network: "cardano-mainnet".into(),
            health_poll_interval: Duration::from_secs(10),
            cors_allow_origin: "*".into(),
            cors_allow_methods: "GET".into(),
            cors_allow_headers: "Content-Type".into(),
            cors_max_age: "86400".into(),
        };
        Arc::new(State::with_config(config).unwrap())
    }

    fn port(name: &str, key: &str, tier: &str) -> OgmiosPort {
        let mut crd = OgmiosPort::new(
            name,
            OgmiosPortSpec {
                network: "mainnet".into(),
                version: 6,
                throughput_tier: tier.into(),
                auth_token: None,
            },
        );
        crd.metadata.namespace = Some("prj-test".into());
        crd.status = Some(OgmiosPortStatus {
            endpoint_url: String::new(),
            authenticated_endpoint_url: String::new(),
            auth_token: key.into(),
        });
        crd
    }

    fn labels(tier: &str) -> ConnectionLabels {
        [
            "ftr-ogmios-v1".into(),
            "ogmios-mainnet-6".into(),
            "host".into(),
            "prj-test.port".into(),
            tier.into(),
        ]
    }

    async fn feed(state: &Arc<State>, events: Vec<Result<Event<OgmiosPort>, watcher::Error>>) {
        run(state.clone(), stream::iter(events)).await;
    }

    /// The 2026-09-07 lockout: a tier change while a connection is open, then
    /// that connection closes. The count must neither reset nor wrap, and the
    /// next connection must be admitted.
    #[tokio::test]
    async fn a_port_update_under_a_live_connection_keeps_its_count() {
        let state = state();
        feed(
            &state,
            vec![Ok(Event::Restarted(vec![port("port", "key", "0")]))],
        )
        .await;

        let connection = state.connections.open("key", labels("0"));
        feed(&state, vec![Ok(Event::Applied(port("port", "key", "3")))]).await;

        assert_eq!(state.get_consumer("key").await.unwrap().tier, "3");
        assert_eq!(state.connections.count("key"), 1);
        assert!(!state.connections.has_capacity("key", MAX_CONNECTIONS));

        drop(connection);

        assert_eq!(state.connections.count("key"), 0);
        assert!(state.connections.has_capacity("key", MAX_CONNECTIONS));
    }

    /// A watch re-list rebuilds the consumer map; every open connection must
    /// still be counted afterwards.
    #[tokio::test]
    async fn a_watcher_restart_under_a_live_connection_keeps_its_count() {
        let state = state();
        feed(
            &state,
            vec![Ok(Event::Restarted(vec![port("port", "key", "0")]))],
        )
        .await;

        let connection = state.connections.open("key", labels("0"));
        feed(
            &state,
            vec![Ok(Event::Restarted(vec![port("port", "key", "0")]))],
        )
        .await;

        assert_eq!(state.connections.count("key"), 1);
        assert!(!state.connections.has_capacity("key", MAX_CONNECTIONS));

        drop(connection);

        assert_eq!(state.connections.count("key"), 0);
        assert!(state.connections.has_capacity("key", MAX_CONNECTIONS));
    }

    /// The routine `410` expiry the API server sends every watcher: the loop
    /// must keep consuming events after it instead of exiting the process.
    #[tokio::test]
    async fn a_watch_error_does_not_stop_the_watcher() {
        let state = state();
        let expired = watcher::Error::WatchError(ErrorResponse {
            status: "Failure".into(),
            message: "too old resource version".into(),
            reason: "Expired".into(),
            code: 410,
        });

        feed(
            &state,
            vec![
                Ok(Event::Restarted(vec![port("port", "key", "0")])),
                Err(expired),
                Ok(Event::Restarted(vec![
                    port("port", "key", "0"),
                    port("other", "other-key", "1"),
                ])),
            ],
        )
        .await;

        assert!(state.get_consumer("key").await.is_some());
        assert!(state.get_consumer("other-key").await.is_some());
    }

    /// A port is created without status. A re-list that catches one in that
    /// window must skip it, not panic the watcher task.
    #[tokio::test]
    async fn a_relist_with_a_port_without_status_skips_it() {
        let state = state();
        let mut pending = port("pending", "unused", "0");
        pending.status = None;

        feed(
            &state,
            vec![
                Ok(Event::Restarted(vec![
                    pending.clone(),
                    port("port", "key", "0"),
                ])),
                Ok(Event::Deleted(pending)),
            ],
        )
        .await;

        assert!(state.get_consumer("key").await.is_some());
        assert_eq!(state.consumers.read().await.len(), 1);
    }
}
