use config::Config;
use futures_util::future;
use metrics::Metrics;
use prometheus::Registry;
use std::error::Error;
use std::sync::Arc;
use tokio::pin;
use tracing::Level;

mod config;
mod metrics;
mod plugins;
mod proxy;
mod utils;

#[derive(Debug, Clone)]
pub struct State {
    config: Config,
    metrics: Metrics,
}
impl State {
    pub fn try_new() -> Result<Self, Box<dyn Error>> {
        let config = Config::try_new()?;
        let metrics = Metrics::try_new(Registry::default())?;

        Ok(Self { config, metrics })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let state = Arc::new(State::try_new()?);

    let metrics = metrics::start(state.clone());
    let proxy_server = proxy::start(state.clone());
    pin!(metrics, proxy_server);

    future::select(metrics, proxy_server).await;

    Ok(())
}
