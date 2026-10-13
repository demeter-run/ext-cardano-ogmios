use std::error::Error;
use std::sync::Arc;
use std::{net::SocketAddr, str::FromStr};

use hyper::server::conn::http1 as http1_server;
use hyper::{body::Incoming, service::service_fn, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use prometheus::{opts, Encoder, IntCounterVec, Registry, TextEncoder};
use tokio::net::TcpListener;
use tracing::{error, info, instrument};

use crate::connections::{ConnectionLabels, Connections, ConnectionsGauge};
use crate::proxy::ProxyRequest;
use crate::utils::{full, ProxyResponse};
use crate::State;

#[derive(Debug, Clone)]
pub struct Metrics {
    registry: Registry,
    pub ws_total_frame: IntCounterVec,
    pub http_total_request: IntCounterVec,
}

impl Metrics {
    pub fn try_new(
        registry: Registry,
        connections: Arc<Connections>,
    ) -> Result<Self, Box<dyn Error>> {
        let ws_total_frame = IntCounterVec::new(
            opts!("ogmios_proxy_ws_total_frame", "total of websocket frame",),
            &["namespace", "instance", "route", "consumer", "tier"],
        )
        .unwrap();

        let http_total_request = IntCounterVec::new(
            opts!("ogmios_proxy_http_total_request", "total of http request",),
            &[
                "namespace",
                "instance",
                "route",
                "status_code",
                "protocol",
                "consumer",
                "tier",
            ],
        )
        .unwrap();

        registry.register(Box::new(ws_total_frame.clone()))?;
        // `ogmios_proxy_total_connections` is read from the same bookkeeping as
        // the connection cap, never maintained alongside it.
        registry.register(Box::new(ConnectionsGauge::new(connections)))?;
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

    pub fn count_ws_total_frame(&self, proxy_req: &ProxyRequest) {
        self.ws_total_frame
            .with_label_values(&[
                &proxy_req.namespace,
                &proxy_req.instance,
                &proxy_req.host,
                &proxy_req.consumer.to_string(),
                &proxy_req.consumer.tier,
            ])
            .inc()
    }

    /// Label values a WebSocket connection is counted under in
    /// `ogmios_proxy_total_connections`.
    pub fn connection_labels(proxy_req: &ProxyRequest) -> ConnectionLabels {
        [
            proxy_req.namespace.clone(),
            proxy_req.instance.clone(),
            proxy_req.host.clone(),
            proxy_req.consumer.to_string(),
            proxy_req.consumer.tier.clone(),
        ]
    }

    pub fn count_http_total_request(&self, proxy_req: &ProxyRequest, status_code: StatusCode) {
        self.http_total_request
            .with_label_values(&[
                &proxy_req.namespace,
                &proxy_req.instance,
                &proxy_req.host,
                &status_code.as_u16().to_string(),
                &proxy_req.protocol.to_string(),
                &proxy_req.consumer.to_string(),
                &proxy_req.consumer.tier,
            ])
            .inc()
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
    state: Arc<State>,
) -> Result<ProxyResponse, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => api_get_metrics(&state).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(full("Not Found"))
            .unwrap()),
    }
}

#[instrument("metrics server", skip_all)]
pub async fn start(state: Arc<State>) {
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
        let state = state.clone();

        let accept_result = listener.accept().await;
        if let Err(err) = accept_result {
            error!(error = err.to_string(), "accept client prometheus server");
            continue;
        }
        let (stream, _) = accept_result.unwrap();

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
