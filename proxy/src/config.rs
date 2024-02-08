use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub proxy_addr: String,
    pub prometheus_addr: String,
    pub ogmios_port: u16,
}

impl Config {
    pub fn new() -> Self {
        Self {
            proxy_addr: env::var("PROXY_ADDR").expect("PROXY_ADDR must be set"),
            prometheus_addr: env::var("PROMETHEUS_ADDR").expect("PROMETHEUS_ADDR must be set"),
            ogmios_port: env::var("OGMIOS_PORT")
                .expect("OGMIOS_PORT must be set")
                .parse()
                .expect("OGMIOS_PORT must a number"),
        }
    }
}
