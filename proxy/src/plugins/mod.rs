use std::fmt::Debug;

use async_trait::async_trait;
use hyper::{body::Incoming, Request, StatusCode};
use thiserror::Error;

use crate::utils::Body;

pub mod auth_dmtr;
pub mod host_key_header;
pub mod rate_limit;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Http Error: {0}")]
    Http(StatusCode, Body),
}

#[async_trait]
pub trait Plugin: Send + Sync {
    fn execute<'a>(
        &self,
        req: &'a mut Request<Incoming>,
    ) -> Result<&'a mut Request<Incoming>, PluginError>;
}

pub fn execute_plugins<'a>(
    req: &'a mut Request<Incoming>,
    plugins: &[Box<dyn Plugin>],
) -> Result<&'a mut Request<Incoming>, PluginError> {
    for plugin in plugins.iter() {
        if let Err(err) = plugin.execute(req) {
            return Err(err);
        }
    }

    Ok(req)
}
