use std::collections::HashMap;

use async_trait::async_trait;
use hyper::{body::Incoming, Request};
use leaky_bucket::RateLimiter;

use super::{Plugin, PluginError};

pub struct RateLimiterPlugin {
    consumers: HashMap<String, RateLimiter>,
}
impl RateLimiterPlugin {
    pub fn new() -> Self {
        let consumers = HashMap::new();
        Self { consumers }
    }
}

#[async_trait]
impl Plugin for RateLimiterPlugin {
    fn execute<'a>(
        &self,
        req: &'a mut Request<Incoming>,
    ) -> Result<&'a mut Request<Incoming>, PluginError> {
        // self.consumers.insert("k".into(), RateLimiter::builder().build());
        Ok(req)
    }
}
