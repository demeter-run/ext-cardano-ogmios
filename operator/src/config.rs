use lazy_static::lazy_static;
use std::{env, time::Duration};

lazy_static! {
    static ref CONTROLLER_CONFIG: Config = Config::from_env();
}

pub fn get_config() -> &'static Config {
    &CONTROLLER_CONFIG
}

#[derive(Debug, Clone)]
pub struct Config {
    pub dns_zone: String,
    pub extension_name: String,
    pub api_key_salt: String,
    pub metrics_delay: Duration,
    pub prometheus_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let dns_zone = env::var("DNS_ZONE").unwrap_or("demeter.run".into());
        let extension_name = env::var("EXTENSION_NAME").unwrap_or("ogmios-m1".into());
        let api_key_salt = env::var("API_KEY_SALT").unwrap_or("ogmios-salt".into());

        let metrics_delay = Duration::from_secs(
            env::var("METRICS_DELAY")
                .expect("METRICS_DELAY must be set")
                .parse::<u64>()
                .expect("METRICS_DELAY must be a number"),
        );
        let prometheus_url = env::var("PROMETHEUS_URL").expect("PROMETHEUS_URL must be set");

        Self {
            dns_zone,
            extension_name,
            api_key_salt,
            metrics_delay,
            prometheus_url,
        }
    }
}
