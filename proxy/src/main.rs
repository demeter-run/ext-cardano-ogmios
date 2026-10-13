use config::Config;
use connections::Connections;
use dotenv::dotenv;
use leaky_bucket::RateLimiter;
use metrics::Metrics;
use operator::{kube::ResourceExt, OgmiosPort};
use prometheus::Registry;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::sync::Arc;
use tiers::Tier;
use tokio::sync::RwLock;
use tracing::Level;

use crate::utils::handle_legacy_networks;

mod auth;
mod config;
mod connections;
mod health;
mod limiter;
mod metrics;
mod proxy;
mod tiers;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let state = Arc::new(State::try_new()?);

    auth::start(state.clone());
    tiers::start(state.clone());

    let metrics = metrics::start(state.clone());
    let proxy_server = proxy::start(state.clone());
    let healthloop = health::start(state.clone());

    tokio::join!(metrics, proxy_server, healthloop);

    Ok(())
}

pub struct State {
    config: Config,
    metrics: Metrics,
    host_regex: Regex,
    consumers: RwLock<HashMap<String, Consumer>>,
    tiers: RwLock<HashMap<String, Tier>>,
    limiter: RwLock<HashMap<String, Vec<Arc<RateLimiter>>>>,
    connections: Arc<Connections>,
    upstream_health: RwLock<bool>,
}
impl State {
    pub fn try_new() -> Result<Self, Box<dyn Error>> {
        Self::with_config(Config::new())
    }

    pub fn with_config(config: Config) -> Result<Self, Box<dyn Error>> {
        let connections = Arc::new(Connections::default());
        let metrics = Metrics::try_new(Registry::default(), connections.clone())?;
        let host_regex = Regex::new(r"([dmtr_]?[\w\d-]+)?\.?.+")?;
        let consumers = Default::default();
        let tiers = Default::default();
        let limiter = Default::default();

        Ok(Self {
            config,
            metrics,
            host_regex,
            consumers,
            tiers,
            limiter,
            connections,
            upstream_health: RwLock::new(false),
        })
    }

    pub async fn get_consumer(&self, key: &str) -> Option<Consumer> {
        self.consumers.read().await.clone().get(key).cloned()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Consumer {
    namespace: String,
    port_name: String,
    tier: String,
    key: String,
    network: String,
    version: String,
}
impl Display for Consumer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.namespace, self.port_name)
    }
}
impl From<&OgmiosPort> for Consumer {
    fn from(value: &OgmiosPort) -> Self {
        let network = handle_legacy_networks(&value.spec.network);
        let version = value.spec.version.to_string();
        let tier = value.spec.throughput_tier.to_string();
        let key = value.status.as_ref().unwrap().auth_token.clone();
        let namespace = value.metadata.namespace.as_ref().unwrap().clone();
        let port_name = value.name_any();

        Self {
            namespace,
            port_name,
            tier,
            key,
            network,
            version,
        }
    }
}
