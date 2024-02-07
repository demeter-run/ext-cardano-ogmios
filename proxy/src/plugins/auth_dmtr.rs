use std::collections::HashMap;

use async_trait::async_trait;
use hyper::{body::Incoming, header::HeaderValue, Request, StatusCode};

use crate::utils::{full, DMTR_API_KEY, DMTR_PROJECT_ID};

use super::{Plugin, PluginError};

#[derive(Debug)]
pub struct DmtrConsumer {
    project_id: String,
}

pub struct AuthDmtrPlugin {
    consumers: HashMap<String, DmtrConsumer>,
}
impl AuthDmtrPlugin {
    pub fn new() -> Self {
        let mut consumers = HashMap::new();

        consumers.insert(
            "dmtr_kupo1d988zmt0g4skjdt6xdxkg5zp23f55ttpxfmsemy6fa".into(),
            DmtrConsumer {
                project_id: "prj-xxxxx-xxxxx".into(),
            },
        );

        Self { consumers }
    }
}

#[async_trait]
impl Plugin for AuthDmtrPlugin {
    fn execute<'a>(
        &self,
        req: &'a mut Request<Incoming>,
    ) -> Result<&'a mut Request<Incoming>, PluginError> {
        let dmtr_key = req
            .headers()
            .get(DMTR_API_KEY)
            .map(|k| k.to_str().unwrap_or_default())
            .unwrap_or_default();

        if let Some(costumer) = self.consumers.get(dmtr_key) {
            req.headers_mut().insert(
                DMTR_PROJECT_ID,
                HeaderValue::from_str(&costumer.project_id).unwrap(),
            );

            return Ok(req);
        }

        return Err(PluginError::Http(
            StatusCode::UNAUTHORIZED,
            full("Unauthorized"),
        ));
    }
}
