use std::{env, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub proxy_addr: String,
    pub proxy_namespace: String,
    pub proxy_tiers_path: PathBuf,
    pub proxy_tiers_poll_interval: Duration,
    pub prometheus_addr: String,
    pub ogmios_port: u16,
    pub ogmios_dns: String,
    pub ssl_crt_path: PathBuf,
    pub ssl_key_path: PathBuf,
    pub network: String,

    // Health endpoint
    pub health_poll_interval: std::time::Duration,

    // CORS configuration
    pub cors_allow_origin: String,
    pub cors_allow_methods: String,
    pub cors_allow_headers: String,
    pub cors_max_age: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            network: env::var("NETWORK").expect("NETWORK must be set"),
            proxy_addr: env::var("PROXY_ADDR").expect("PROXY_ADDR must be set"),
            proxy_namespace: env::var("PROXY_NAMESPACE").unwrap_or("ftr-ogmios-v1".into()),
            proxy_tiers_path: env::var("PROXY_TIERS_PATH")
                .map(|v| v.into())
                .expect("PROXY_TIERS_PATH must be set"),
            proxy_tiers_poll_interval: env::var("PROXY_TIERS_POLL_INTERVAL")
                .map(|v| {
                    Duration::from_secs(
                        v.parse::<u64>()
                            .expect("PROXY_TIERS_POLL_INTERVAL must be a number in seconds. eg: 2"),
                    )
                })
                .unwrap_or(Duration::from_secs(2)),
            prometheus_addr: env::var("PROMETHEUS_ADDR").expect("PROMETHEUS_ADDR must be set"),
            ssl_crt_path: env::var("SSL_CRT_PATH")
                .map(|e| e.into())
                .expect("SSL_CRT_PATH must be set"),
            ssl_key_path: env::var("SSL_KEY_PATH")
                .map(|e| e.into())
                .expect("SSL_KEY_PATH must be set"),
            ogmios_port: env::var("OGMIOS_PORT")
                .expect("OGMIOS_PORT must be set")
                .parse()
                .expect("OGMIOS_PORT must a number"),
            ogmios_dns: env::var("OGMIOS_DNS").expect("OGMIOS_DNS must be set"),
            health_poll_interval: env::var("HEALTH_POLL_INTERVAL")
                .map(|v| {
                    Duration::from_secs(
                        v.parse::<u64>()
                            .expect("HEALTH_POLL_INTERVAL must be a number in seconds. eg: 2"),
                    )
                })
                .unwrap_or(Duration::from_secs(10)),
            cors_allow_origin: env::var("CORS_ALLOW_ORIGIN").unwrap_or("*".into()),
            cors_allow_methods: env::var("CORS_ALLOW_METHODS")
                .unwrap_or("GET, POST, PUT, DELETE, OPTIONS".into()),
            cors_allow_headers: env::var("CORS_ALLOW_HEADERS")
                .unwrap_or("Content-Type, Authorization, X-Requested-With, dmtr-api-key".into()),
            cors_max_age: env::var("CORS_MAX_AGE").unwrap_or("86400".into()),
        }
    }

    pub fn instance(&self, version: &str) -> String {
        format!(
            "ogmios-{}-{}.{}:{}",
            self.network, version, self.ogmios_dns, self.ogmios_port
        )
    }
}
