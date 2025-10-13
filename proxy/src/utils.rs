use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{body::Incoming, Request, Response};
use lazy_static::lazy_static;
use std::collections::HashMap;

pub const DMTR_API_KEY: &str = "dmtr-api-key";
pub type Body = BoxBody<Bytes, hyper::Error>;
pub type ProxyResponse = Response<Body>;

lazy_static! {
    static ref LEGACY_NETWORKS: HashMap<&'static str, String> = {
        let mut m = HashMap::new();
        m.insert("mainnet", "cardano-mainnet".into());
        m.insert("preprod", "cardano-preprod".into());
        m.insert("preview", "cardano-preview".into());
        m
    };
}

pub fn handle_legacy_networks(network: &str) -> String {
    let default = network.to_string();
    LEGACY_NETWORKS.get(network).unwrap_or(&default).to_string()
}

pub fn full<T: Into<Bytes>>(chunk: T) -> Body {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

pub fn get_header(req: &Request<Incoming>, key: &str) -> Option<String> {
    req.headers()
        .get(key)
        .and_then(|h| h.to_str().ok().map(|v| v.to_string()))
}
