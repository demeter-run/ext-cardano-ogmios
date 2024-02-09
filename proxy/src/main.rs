use config::Config;
use dotenv::dotenv;
use metrics::Metrics;
use prometheus::Registry;
use regex::Regex;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::Level;

mod config;
mod metrics;
mod proxy;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let state = Arc::new(RwLock::new(State::try_new()?));

    let metrics = metrics::start(state.clone());
    let proxy_server = proxy::start(state.clone());

    let result = tokio::join!(metrics, proxy_server);
    result.0?;
    result.1?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct State {
    config: Config,
    metrics: Metrics,
    host_regex: Regex,
}
impl State {
    pub fn try_new() -> Result<Self, Box<dyn Error>> {
        let config = Config::new();
        let metrics = Metrics::try_new(Registry::default())?;
        let host_regex = Regex::new(r"(dmtr_[\w\d-]+)?\.?([\w]+).+-([\w\d]+).+")?;

        Ok(Self {
            config,
            metrics,
            host_regex,
        })
    }
}
