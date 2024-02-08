use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{body::Incoming, header::UPGRADE, Request, Response};

pub const DMTR_API_KEY: &str = "dmtr-api-key";

pub type Body = BoxBody<Bytes, hyper::Error>;
pub type ProxyResponse = Response<Body>;

pub fn full<T: Into<Bytes>>(chunk: T) -> Body {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

pub fn get_header(req: &mut Request<Incoming>, key: &str) -> Option<String> {
    req.headers()
        .get(key)
        .and_then(|h| h.to_str().ok().and_then(|v| Some(v.to_string())))
}

pub enum Protocol {
    Http,
    Websocket,
}
impl Protocol {
    pub fn match_protocol(req: &mut Request<Incoming>) -> Self {
        if get_header(req, UPGRADE.as_str())
            .map(|h| h.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false)
        {
            return Self::Websocket;
        }

        Self::Http
    }
}
