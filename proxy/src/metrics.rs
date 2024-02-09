use std::error::Error;
use std::sync::Arc;
use std::{net::SocketAddr, str::FromStr};

use hyper::server::conn::http1 as http1_server;
use hyper::{body::Incoming, service::service_fn, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use prometheus::{opts, Encoder, IntCounterVec, Registry, TextEncoder};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
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

async fn routes_match(
    req: Request<Incoming>,
    rw_state: Arc<RwLock<State>>,
) -> Result<ProxyResponse, hyper::Error> {
    let state = rw_state.read().await.clone();

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => api_get_metrics(&state).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(full("Not Found"))
            .unwrap()),
    }
}

pub async fn start(rw_state: Arc<RwLock<State>>) {
    let state = rw_state.read().await.clone();

    let addr_result = SocketAddr::from_str(&state.config.prometheus_addr);
    if let Err(err) = addr_result {
        error!(error = err.to_string(), "invalid prometheus addr");
        std::process::exit(1);
    }
    let addr = addr_result.unwrap();

    let listener_result = TcpListener::bind(addr).await;
    if let Err(err) = listener_result {
        error!(
            error = err.to_string(),
            "fail to bind tcp prometheus server listener"
        );
        std::process::exit(1);
    }
    let listener = listener_result.unwrap();

    info!(addr = state.config.prometheus_addr, "metrics listening");

    loop {
        let rw_state = rw_state.clone();

        let accept_result = listener.accept().await;
        if let Err(err) = accept_result {
            error!(error = err.to_string(), "accept client prometheus server");
            continue;
        }
        let (stream, _) = accept_result.unwrap();

        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            let service = service_fn(move |req| routes_match(req, rw_state.clone()));

            if let Err(err) = http1_server::Builder::new()
                .serve_connection(io, service)
                .await
            {
                error!(error = err.to_string(), "failed metrics server connection");
            }
        });
    }
}
