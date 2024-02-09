use futures_util::{future, stream::TryStreamExt, StreamExt};
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::body::Incoming;
use hyper::client::conn::http1 as http1_client;
use hyper::header::{
    HeaderValue, CONNECTION, HOST, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, UPGRADE,
};
use hyper::server::conn::http1 as http1_server;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::{connect_async, WebSocketStream};
use tracing::{error, info};
use url::Url;

use crate::utils::{full, get_header, Protocol, ProxyResponse, DMTR_API_KEY};
use crate::State;

pub async fn start(rw_state: Arc<RwLock<State>>) {
    let state = rw_state.read().await.clone();

    let addr_result = SocketAddr::from_str(&state.config.proxy_addr);
    if let Err(err) = addr_result {
        error!(error = err.to_string(), "invalid proxy addr");
        std::process::exit(1);
    }
    let addr = addr_result.unwrap();

    let listener_result = TcpListener::bind(addr).await;
    if let Err(err) = listener_result {
        error!(error = err.to_string(), "fail to bind tcp server listener");
        std::process::exit(1);
    }
    let listener = listener_result.unwrap();

    info!(addr = state.config.proxy_addr, "proxy listening");

    loop {
        let rw_state = rw_state.clone();
        let accept_result = listener.accept().await;
        if let Err(err) = accept_result {
            error!(error = err.to_string(), "fail to accept client");
            continue;
        }
        let (stream, _) = accept_result.unwrap();

        tokio::task::spawn(async move {
            let io = TokioIo::new(stream);

            let service = service_fn(move |req| handle(req, rw_state.clone()));

            if let Err(err) = http1_server::Builder::new()
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                error!(error = err.to_string(), "failed proxy server connection");
            }
        });
    }
}

async fn handle(
    mut req: Request<Incoming>,
    rw_state: Arc<RwLock<State>>,
) -> Result<ProxyResponse, hyper::Error> {
    let state = rw_state.read().await.clone();

    let port_host = get_header(&req, HOST.as_str()).unwrap().to_string();

    let captures = state.host_regex.captures(&port_host).unwrap();
    let network: &str = captures.get(2).unwrap().into();
    let version: &str = captures.get(3).unwrap().into();
    let ogmios_host = format!("ogmios-{network}-{version}:{}", state.config.ogmios_port);

    if let Some(key) = captures.get(1) {
        req.headers_mut()
            .insert(DMTR_API_KEY, HeaderValue::from_str(key.as_str()).unwrap());
    }

    let token = get_header(&req, DMTR_API_KEY).unwrap_or_default();
    let consumer = state.get_auth_token(network, version, &token);
    if consumer.is_none() {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(full("Unauthorized"))
            .unwrap());
    }

    dbg!(consumer.unwrap().namespace);

    match Protocol::match_protocol(&mut req) {
        Protocol::Http => handle_http(req, rw_state.clone(), &ogmios_host).await,
        Protocol::Websocket => handle_websocket(req, rw_state.clone(), &ogmios_host).await,
    }
}

async fn handle_http(
    req: Request<Incoming>,
    rw_state: Arc<RwLock<State>>,
    ogmios_host: &str,
) -> Result<ProxyResponse, hyper::Error> {
    let state = rw_state.read().await.clone();

    state.metrics.count_http_total_request("dmtr_project_id");

    let stream = TcpStream::connect(ogmios_host).await.unwrap();
    let io: TokioIo<TcpStream> = TokioIo::new(stream);

    let (mut sender, conn) = http1_client::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .handshake(io)
        .await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let resp = sender.send_request(req).await?;
    Ok(resp.map(|b| b.boxed()))
}

async fn handle_websocket(
    mut req: Request<Incoming>,
    rw_state: Arc<RwLock<State>>,
    ogmios_host: &str,
) -> Result<ProxyResponse, hyper::Error> {
    let headers = req.headers();
    let upgrade = HeaderValue::from_static("Upgrade");
    let websocket = HeaderValue::from_static("websocket");
    let key = headers.get(SEC_WEBSOCKET_KEY);
    let derived = key.map(|k| derive_accept_key(k.as_bytes()));
    let version = req.version();
    let ogmios_host = ogmios_host.to_string();

    tokio::task::spawn(async move {
        match hyper::upgrade::on(&mut req).await {
            Ok(upgraded) => {
                let state = rw_state.read().await.clone();

                let upgraded = TokioIo::new(upgraded);
                let client_stream =
                    WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await;
                let (client_outgoing, client_incoming) = client_stream.split();

                let url = Url::parse(&format!("ws://{ogmios_host}{}", req.uri())).unwrap();
                let connection_result = connect_async(url).await;
                if let Err(err) = connection_result {
                    error!(error = err.to_string(), "fail to connect to the host");
                    return;
                }
                let (host_stream, _) = connection_result.unwrap();
                let (host_outgoing, host_incoming) = host_stream.split();

                let client_in = client_incoming
                    .inspect_ok(|_| {
                        state.metrics.count_ws_total_frame("");
                    })
                    .forward(host_outgoing);
                let host_in = host_incoming.forward(client_outgoing);

                future::select(client_in, host_in).await;
            }
            Err(err) => {
                error!(error = err.to_string(), "upgrade error");
            }
        }
    });

    let mut res = Response::new(BoxBody::default());
    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    *res.version_mut() = version;
    res.headers_mut().append(CONNECTION, upgrade);
    res.headers_mut().append(UPGRADE, websocket);
    res.headers_mut()
        .append(SEC_WEBSOCKET_ACCEPT, derived.unwrap().parse().unwrap());

    Ok(res)
}
