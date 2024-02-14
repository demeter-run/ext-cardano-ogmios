use std::sync::Arc;

use kube::ResourceExt;
use prometheus::{opts, IntCounterVec, Registry};
use tracing::{info, instrument};

use crate::{Error, OgmiosPort, State};

#[derive(Clone)]
pub struct Metrics {
    pub reconcile_failures: IntCounterVec,
}

impl Default for Metrics {
    fn default() -> Self {
        let reconcile_failures = IntCounterVec::new(
            opts!(
                "ogmios_operator_reconciliation_errors_total",
                "reconciliation errors",
            ),
            &["instance", "error"],
        )
        .unwrap();

        Metrics { reconcile_failures }
    }
}

impl Metrics {
    pub fn register(self, registry: &Registry) -> Result<Self, prometheus::Error> {
        registry.register(Box::new(self.reconcile_failures.clone()))?;

        Ok(self)
    }

    pub fn reconcile_failure(&self, crd: &OgmiosPort, e: &Error) {
        self.reconcile_failures
            .with_label_values(&[crd.name_any().as_ref(), e.metric_label().as_ref()])
            .inc()
    }
}

#[instrument("metrics collector run", skip_all)]
pub async fn run_metrics_collector(_state: Arc<State>) {
    tokio::spawn(async move {
        info!("collecting metrics running");
    });
}

