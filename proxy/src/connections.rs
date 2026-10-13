use prometheus::core::{Collector, Desc};
use prometheus::proto::MetricFamily;
use prometheus::{opts, IntGaugeVec};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use tracing::warn;

/// Label values of `ogmios_proxy_total_connections`, in the order of
/// [`LABELS`]: namespace, instance, route, consumer, tier.
pub type ConnectionLabels = [String; 5];

const LABELS: [&str; 5] = ["namespace", "instance", "route", "consumer", "tier"];

/// The one bookkeeping of open WebSocket connections.
///
/// The connection cap and the `ogmios_proxy_total_connections` gauge both read
/// from here, so they cannot disagree. It is deliberately separate from the
/// consumer map: that map is rebuilt from the CRDs on every watcher event, and
/// a CRD update changes a port's tier or network, never how many sockets it has
/// open. Keeping the count on the consumer entry is what let a tier change
/// reset it under live connections, which then wrapped on their disconnect.
///
/// Counts are keyed by consumer key (what the cap applies to) and then by the
/// gauge's label values at connect time, so a connection is released under the
/// same labels it was opened with even if the tier changes in between. Entries
/// are kept at zero rather than removed, as the gauge series always were.
#[derive(Default)]
pub struct Connections {
    open: Mutex<HashMap<String, HashMap<ConnectionLabels, usize>>>,
}

impl Connections {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, HashMap<ConnectionLabels, usize>>> {
        // Every critical section is a single map update, so a poisoned lock
        // still guards consistent data.
        self.open
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Record an open connection. It stays counted until the returned guard is
    /// dropped.
    pub fn open(self: &Arc<Self>, key: &str, labels: ConnectionLabels) -> Connection {
        *self
            .lock()
            .entry(key.to_string())
            .or_default()
            .entry(labels.clone())
            .or_default() += 1;

        Connection {
            connections: self.clone(),
            key: key.to_string(),
            labels,
        }
    }

    fn close(&self, key: &str, labels: &ConnectionLabels) {
        let mut open = self.lock();
        let count = open
            .get_mut(key)
            .and_then(|by_labels| by_labels.get_mut(labels));
        match count {
            Some(count) if *count > 0 => *count -= 1,
            _ => warn!(
                consumer = labels[3],
                "closing a connection that was never counted open; keeping the count at zero"
            ),
        }
    }

    /// Connections open for a consumer key, across all hostnames and tiers.
    pub fn count(&self, key: &str) -> usize {
        self.lock()
            .get(key)
            .map(|by_labels| by_labels.values().sum())
            .unwrap_or_default()
    }

    /// Whether a consumer key may open one more connection under a cap.
    pub fn has_capacity(&self, key: &str, max_connections: usize) -> bool {
        self.count(key) < max_connections
    }
}

/// An open connection. Dropping it closes it in the bookkeeping, so it is
/// released exactly once on every exit path, panics included.
pub struct Connection {
    connections: Arc<Connections>,
    key: String,
    labels: ConnectionLabels,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.connections.close(&self.key, &self.labels);
    }
}

/// Exposes [`Connections`] as the `ogmios_proxy_total_connections` gauge,
/// computed at scrape time rather than maintained alongside.
pub struct ConnectionsGauge {
    connections: Arc<Connections>,
    template: IntGaugeVec,
}

impl ConnectionsGauge {
    pub fn new(connections: Arc<Connections>) -> Self {
        Self {
            connections,
            template: gauge_vec(),
        }
    }
}

fn gauge_vec() -> IntGaugeVec {
    IntGaugeVec::new(
        opts!(
            "ogmios_proxy_total_connections",
            "total of websocket connection",
        ),
        &LABELS,
    )
    .unwrap()
}

impl Collector for ConnectionsGauge {
    fn desc(&self) -> Vec<&Desc> {
        self.template.desc()
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let gauge = gauge_vec();
        for by_labels in self.connections.lock().values() {
            for (labels, count) in by_labels {
                let values: Vec<&str> = labels.iter().map(String::as_str).collect();
                gauge.with_label_values(&values).set(*count as i64);
            }
        }
        gauge.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::Registry;

    fn labels(consumer: &str, tier: &str) -> ConnectionLabels {
        [
            "ftr-ogmios-v1".into(),
            "ogmios-mainnet-6".into(),
            "host".into(),
            consumer.into(),
            tier.into(),
        ]
    }

    /// Every `ogmios_proxy_total_connections` sample, as (consumer, tier, value).
    fn scrape(registry: &Registry) -> Vec<(String, String, i64)> {
        let mut samples: Vec<_> = registry
            .gather()
            .iter()
            .filter(|family| family.get_name() == "ogmios_proxy_total_connections")
            .flat_map(|family| family.get_metric().to_vec())
            .map(|metric| {
                let label = |name: &str| {
                    metric
                        .get_label()
                        .iter()
                        .find(|pair| pair.get_name() == name)
                        .unwrap()
                        .get_value()
                        .to_string()
                };
                (
                    label("consumer"),
                    label("tier"),
                    metric.get_gauge().get_value() as i64,
                )
            })
            .collect();
        samples.sort();
        samples
    }

    #[test]
    fn the_cap_and_the_gauge_read_the_same_count() {
        let connections = Arc::new(Connections::default());
        let registry = Registry::new();
        registry
            .register(Box::new(ConnectionsGauge::new(connections.clone())))
            .unwrap();

        let first = connections.open("key-a", labels("ns.port-a", "0"));
        let second = connections.open("key-a", labels("ns.port-a", "3"));
        let _other = connections.open("key-b", labels("ns.port-b", "1"));

        assert_eq!(connections.count("key-a"), 2);
        assert!(!connections.has_capacity("key-a", 2));
        assert!(connections.has_capacity("key-a", 3));
        assert_eq!(
            scrape(&registry),
            vec![
                ("ns.port-a".into(), "0".into(), 1),
                ("ns.port-a".into(), "3".into(), 1),
                ("ns.port-b".into(), "1".into(), 1),
            ]
        );

        drop(first);
        drop(second);

        assert_eq!(connections.count("key-a"), 0);
        assert!(connections.has_capacity("key-a", 1));
        assert_eq!(
            scrape(&registry),
            vec![
                ("ns.port-a".into(), "0".into(), 0),
                ("ns.port-a".into(), "3".into(), 0),
                ("ns.port-b".into(), "1".into(), 1),
            ]
        );
    }

    /// An unmatched close is a bookkeeping bug, but it must never take the
    /// count below zero: that is what wrapped `usize` and locked ports out.
    #[test]
    fn an_unmatched_close_saturates_at_zero() {
        let connections = Arc::new(Connections::default());

        connections.close("key-a", &labels("ns.port-a", "0"));
        let connection = connections.open("key-a", labels("ns.port-a", "0"));
        connections.close("key-a", &labels("ns.port-a", "0"));
        drop(connection);

        assert_eq!(connections.count("key-a"), 0);
        assert!(connections.has_capacity("key-a", 1));
    }
}
