use std::{env, error::Error};

use serde::Deserialize;

use crate::plugins::{
    auth_dmtr::AuthDmtrPlugin, host_key_header::HostKeyHeaderPlugin, rate_limit::RateLimiterPlugin,
    Plugin,
};

#[derive(Debug, Clone, Deserialize)]
pub enum Protocol {
    Http,
    Websocket,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    pub path: String,
    pub host: String,
    pub port: u16,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "type", content = "priority")]
pub enum HttpPluginsAvailable {
    AuthDmtrPlugin(u16),
    RateLimiterPlugin(u16),
    HostKeyHeaderPlugin(u16),
}
impl HttpPluginsAvailable {
    pub fn priority(&self) -> u16 {
        match self {
            HttpPluginsAvailable::AuthDmtrPlugin(priority) => priority.clone(),
            HttpPluginsAvailable::RateLimiterPlugin(priority) => priority.clone(),
            HttpPluginsAvailable::HostKeyHeaderPlugin(priority) => priority.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "type", content = "priority")]
pub enum WebsocketPluginsAvailable {
    RateLimiterPlugin(u16),
}
impl WebsocketPluginsAvailable {
    pub fn priority(&self) -> u16 {
        match self {
            WebsocketPluginsAvailable::RateLimiterPlugin(priority) => priority.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub proxy_addr: String,
    pub prometheus_addr: String,
    pub routes: Vec<Route>,
    pub http_plugins: Vec<HttpPluginsAvailable>,
    pub websocket_plugins: Vec<WebsocketPluginsAvailable>,
}

impl Config {
    pub fn try_new() -> Result<Self, Box<dyn Error>> {
        let path = env::var("PROXY_CONFIG_PATH").unwrap_or("proxy.toml".into());

        let config = config::Config::builder()
            .add_source(config::File::with_name(&path).required(true))
            .build()?
            .try_deserialize()?;

        Ok(config)
    }

    pub fn match_route(&self, path: &str) -> Option<&Route> {
        self.routes.iter().find(|r| r.path.eq(path))
    }

    pub fn mount_plugins(&self) -> (Vec<Box<dyn Plugin>>, Vec<Box<dyn Plugin>>) {
        let mut http_plugins: Vec<Box<dyn Plugin>> = Vec::new();

        let mut config_http_plugins = self.http_plugins.clone();
        config_http_plugins.sort_by_key(|p| p.priority());

        for plugin in config_http_plugins.iter() {
            match plugin {
                HttpPluginsAvailable::AuthDmtrPlugin(_) => {
                    http_plugins.push(Box::new(AuthDmtrPlugin::new()))
                }
                HttpPluginsAvailable::RateLimiterPlugin(_) => {
                    http_plugins.push(Box::new(RateLimiterPlugin::new()))
                }
                HttpPluginsAvailable::HostKeyHeaderPlugin(_) => {
                    http_plugins.push(Box::new(HostKeyHeaderPlugin::new()))
                }
            }
        }

        let mut websocket_plugins: Vec<Box<dyn Plugin>> = Vec::new();

        let mut config_websocket_plugins = self.websocket_plugins.clone();
        config_websocket_plugins.sort_by_key(|p| p.priority());

        for plugin in self.websocket_plugins.iter() {
            match plugin {
                WebsocketPluginsAvailable::RateLimiterPlugin(_) => {
                    websocket_plugins.push(Box::new(RateLimiterPlugin::new()))
                }
            }
        }

        (http_plugins, websocket_plugins)
    }
}
