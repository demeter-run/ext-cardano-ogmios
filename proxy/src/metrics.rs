use std::error::Error;
use std::sync::Arc;
use std::{net::SocketAddr, str::FromStr};

use hyper::server::conn::http1 as http1_server;
use hyper::{body::Incoming, service::service_fn, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use prometheus::{opts, Encoder, IntCounterVec, IntGaugeVec, Registry, TextEncoder};
use tokio::net::TcpListener;
use tracing::{error, info, instrument};

use crate::proxy::ProxyRequest;
use crate::rpc::{is_heavy, Classification, Transport};
use crate::utils::{full, ProxyResponse};
use crate::State;

#[derive(Debug, Clone)]
pub struct Metrics {
    registry: Registry,
    pub ws_total_frame: IntCounterVec,
    pub ws_total_connection: IntGaugeVec,
    pub http_total_request: IntCounterVec,
    pub rpc_requests: IntCounterVec,
    pub rpc_consumer_requests: IntCounterVec,
    pub rpc_consumer_heavy_requests: IntCounterVec,
}

impl Metrics {
    pub fn try_new(registry: Registry) -> Result<Self, Box<dyn Error>> {
        let ws_total_frame = IntCounterVec::new(
            opts!("ogmios_proxy_ws_total_frame", "total of websocket frame",),
            &["namespace", "instance", "route", "consumer", "tier"],
        )
        .unwrap();

        let ws_total_connection = IntGaugeVec::new(
            opts!(
                "ogmios_proxy_total_connections",
                "total of websocket connection",
            ),
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

        // Request classification. These avoid the label names Prometheus
        // target labels already use (namespace, instance, job, pod, container,
        // endpoint), and only the low-cardinality families carry `consumer`.
        let rpc_requests = IntCounterVec::new(
            opts!(
                "ogmios_proxy_rpc_requests_total",
                "client JSON-RPC messages by method and shape",
            ),
            &[
                "network",
                "version",
                "tier",
                "transport",
                "method",
                "shape",
                "heavy",
            ],
        )?;

        let rpc_consumer_requests = IntCounterVec::new(
            opts!(
                "ogmios_proxy_rpc_consumer_requests_total",
                "client JSON-RPC messages by consumer and method family",
            ),
            &["consumer", "network", "version", "family"],
        )?;

        let rpc_consumer_heavy_requests = IntCounterVec::new(
            opts!(
                "ogmios_proxy_rpc_consumer_heavy_requests_total",
                "client JSON-RPC messages of a heavy class by consumer",
            ),
            &["consumer", "network", "version", "method", "shape"],
        )?;

        registry.register(Box::new(ws_total_frame.clone()))?;
        registry.register(Box::new(ws_total_connection.clone()))?;
        registry.register(Box::new(http_total_request.clone()))?;
        registry.register(Box::new(rpc_requests.clone()))?;
        registry.register(Box::new(rpc_consumer_requests.clone()))?;
        registry.register(Box::new(rpc_consumer_heavy_requests.clone()))?;

        Ok(Metrics {
            registry,
            ws_total_frame,
            ws_total_connection,
            http_total_request,
            rpc_requests,
            rpc_consumer_requests,
            rpc_consumer_heavy_requests,
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

    pub fn inc_ws_total_connection(&self, proxy_req: &ProxyRequest) {
        self.ws_total_connection
            .with_label_values(&[
                &proxy_req.namespace,
                &proxy_req.instance,
                &proxy_req.host,
                &proxy_req.consumer.to_string(),
                &proxy_req.consumer.tier,
            ])
            .inc()
    }

    pub fn dec_ws_total_connection(&self, proxy_req: &ProxyRequest) {
        self.ws_total_connection
            .with_label_values(&[
                &proxy_req.namespace,
                &proxy_req.instance,
                &proxy_req.host,
                &proxy_req.consumer.to_string(),
                &proxy_req.consumer.tier,
            ])
            .dec()
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

    /// Counts one classified client message. `consumer` is the consumer's
    /// label value, resolved once per connection or request by the caller.
    pub fn count_rpc_request(
        &self,
        proxy_req: &ProxyRequest,
        consumer: &str,
        transport: Transport,
        classification: &Classification,
    ) {
        let network = proxy_req.consumer.network.as_str();
        let version = proxy_req.consumer.version.as_str();
        let method = classification.method.as_str();
        let shape = classification.shape.as_str();
        let heavy = is_heavy(classification);

        self.rpc_requests
            .with_label_values(&[
                network,
                version,
                &proxy_req.consumer.tier,
                transport.as_str(),
                method,
                shape,
                if heavy { "true" } else { "false" },
            ])
            .inc();

        self.rpc_consumer_requests
            .with_label_values(&[
                consumer,
                network,
                version,
                classification.method.family().as_str(),
            ])
            .inc();

        if heavy {
            self.rpc_consumer_heavy_requests
                .with_label_values(&[consumer, network, version, method, shape])
                .inc();
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::Protocol;
    use crate::rpc::classify;
    use crate::Consumer;

    fn exposition(metrics: &Metrics) -> String {
        let mut buffer = vec![];
        TextEncoder::new()
            .encode(&metrics.metrics_collected(), &mut buffer)
            .unwrap();
        String::from_utf8(buffer).unwrap()
    }

    fn proxy_request() -> ProxyRequest {
        ProxyRequest {
            namespace: "ftr-ogmios-v1".into(),
            host: "key.cardano-preprod-v6.ogmios-m1.demeter.run".into(),
            instance: "ogmios-cardano-preprod-6".into(),
            consumer: Consumer {
                namespace: "prj-a".into(),
                port_name: "port-b".into(),
                tier: "1".into(),
                network: "cardano-preprod".into(),
                version: "6".into(),
                ..Default::default()
            },
            protocol: Protocol::Websocket,
        }
    }

    #[test]
    fn rpc_families_have_no_series_until_a_message_is_counted() {
        let metrics = Metrics::try_new(Registry::default()).unwrap();
        assert!(!exposition(&metrics).contains("ogmios_proxy_rpc"));
    }

    #[test]
    fn a_heavy_message_is_counted_in_all_three_families() {
        let metrics = Metrics::try_new(Registry::default()).unwrap();
        let utxo = classify(br#"{"method":"queryLedgerState/utxo","params":{"addresses":["a"]}}"#);
        metrics.count_rpc_request(
            &proxy_request(),
            "prj-a.port-b",
            Transport::Websocket,
            &utxo,
        );

        let text = exposition(&metrics);
        assert!(text.contains(r#"ogmios_proxy_rpc_requests_total{heavy="true",method="queryLedgerState/utxo",network="cardano-preprod",shape="addresses/1",tier="1",transport="websocket",version="6"} 1"#), "{text}");
        assert!(text.contains(r#"ogmios_proxy_rpc_consumer_requests_total{consumer="prj-a.port-b",family="ledger-state",network="cardano-preprod",version="6"} 1"#), "{text}");
        assert!(text.contains(r#"ogmios_proxy_rpc_consumer_heavy_requests_total{consumer="prj-a.port-b",method="queryLedgerState/utxo",network="cardano-preprod",shape="addresses/1",version="6"} 1"#), "{text}");
    }

    #[test]
    fn a_light_message_has_no_heavy_series() {
        let metrics = Metrics::try_new(Registry::default()).unwrap();
        let tip = classify(br#"{"method":"queryNetwork/tip"}"#);
        metrics.count_rpc_request(&proxy_request(), "prj-a.port-b", Transport::Http, &tip);

        let text = exposition(&metrics);
        assert!(
            text.contains(r#"heavy="false",method="queryNetwork/tip""#),
            "{text}"
        );
        assert!(text.contains(r#"transport="http""#), "{text}");
        assert!(
            !text.contains("ogmios_proxy_rpc_consumer_heavy_requests_total{"),
            "{text}"
        );
    }
}
