use std::error::Error;
use std::sync::Arc;
use std::{net::SocketAddr, str::FromStr};

use hyper::server::conn::http1 as http1_server;
use hyper::{body::Incoming, service::service_fn, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use prometheus::{opts, Encoder, IntCounterVec, Registry, TextEncoder};
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::utils::{full, ProxyResponse};
use crate::State;

#[derive(Debug, Clone)]
pub struct Metrics {
    registry: Registry,
    pub ws_total_frame: IntCounterVec,
    pub http_total_request: IntCounterVec,
}

impl Metrics {
    pub fn try_new(registry: Registry) -> Result<Self, Box<dyn Error>> {
        let ws_total_frame = IntCounterVec::new(
            opts!("proxy_ws_total_frame", "total of websocket frame",),
            &["dmtr_key"],
        )
        .unwrap();
        let http_total_request = IntCounterVec::new(
            opts!("proxy_http_total_request", "total of http request",),
            &["dmtr_key"],
        )
        .unwrap();

        registry.register(Box::new(ws_total_frame.clone()))?;
        registry.register(Box::new(http_total_request.clone()))?;

        Ok(Metrics {
            registry,
            ws_total_frame,
            http_total_request,
        })
    }

    pub fn metrics_collected(&self) -> Vec<prometheus::proto::MetricFamily> {
        self.registry.gather()
    }

    pub fn count_ws_total_frame(&self, dmtr_key: &str) {
        self.ws_total_frame.with_label_values(&[dmtr_key]).inc()
    }

    pub fn count_http_total_request(&self, dmtr_key: &str) {
        self.http_total_request.with_label_values(&[dmtr_key]).inc()
    }
}

async fn api_get_metrics(state: &State) -> Result<ProxyResponse, hyper::Error> {
    let metrics = state.metrics.metrics_collected();
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder.encode(&metrics, &mut buffer).unwrap();

    let res = Response::builder().body(full(buffer)).unwrap();
    Ok(res)
}

async fn routes_match(req: Request<Incoming>, state: Arc<State>) -> Result<ProxyResponse, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => api_get_metrics(&state).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(full("Not Found"))
            .unwrap()),
    }
}

pub async fn start(state: Arc<State>) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from_str(&state.config.prometheus_addr)?;
    let listener = TcpListener::bind(addr).await?;
    info!(addr = state.config.prometheus_addr, "metrics listening");

    loop {
        let state = state.clone();

        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            let service = service_fn(move |req| routes_match(req, state.clone()));

            if let Err(err) = http1_server::Builder::new()
                .serve_connection(io, service)
                .await
            {
                error!(error = err.to_string(), "failed metrics server connection");
            }
        });
    }
}
