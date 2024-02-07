use async_trait::async_trait;
use hyper::{
    body::Incoming,
    header::{HeaderValue, HOST},
    Request,
};
use regex::Regex;

use crate::utils::DMTR_API_KEY;

use super::{Plugin, PluginError};

#[derive(Debug)]
pub struct HostKeyHeaderPlugin {
    regex: Regex,
}
impl HostKeyHeaderPlugin {
    pub fn new() -> Self {
        let regex = Regex::new(r"(dmtr_[\w\d-]+)\.[\w+].+").unwrap();
        Self { regex }
    }
}

#[async_trait]
impl Plugin for HostKeyHeaderPlugin {
    fn execute<'a>(
        &self,
        req: &'a mut Request<Incoming>,
    ) -> Result<&'a mut Request<Incoming>, PluginError> {
        let host = req
            .headers()
            .get(HOST)
            .map(|k| k.to_str().unwrap_or_default())
            .unwrap_or_default()
            .to_string();

        if let Some(captures) = self.regex.captures(&host) {
            let dmtr_key = captures.get(1).unwrap().as_str();

            req.headers_mut()
                .insert(DMTR_API_KEY, HeaderValue::from_str(dmtr_key).unwrap());
        }

        Ok(req)
    }
}
