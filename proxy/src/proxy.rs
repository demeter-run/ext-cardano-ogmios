use futures_util::{future, stream::TryStreamExt, StreamExt};
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::body::Incoming;
use hyper::client::conn::http1 as http1_client;
use hyper::header::{HeaderValue, CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, UPGRADE};
use hyper::server::conn::http1 as http1_server;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::{connect_async, WebSocketStream};
use tracing::{error, info};
use url::Url;

use crate::config::{Protocol, Route};
use crate::plugins::{execute_plugins, Plugin, PluginError};
use crate::utils::{full, ProxyResponse, DMTR_PROJECT_ID};
use crate::State;

pub async fn start(state: Arc<State>) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from_str(&state.config.proxy_addr)?;
    let listener = TcpListener::bind(addr).await?;
    info!(addr = state.config.proxy_addr, "proxy listening");

    let (http_plugins, websocket_plugins) = state.config.mount_plugins();
    let http_plugins = Arc::new(http_plugins);
    let websocket_plugins = Arc::new(websocket_plugins);

    loop {
        let state = state.clone();
        let http_plugins = http_plugins.clone();
        let websocket_plugins = websocket_plugins.clone();

        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            let io = TokioIo::new(stream);

            let service = service_fn(move |req| {
                handle(
                    req,
                    state.clone(),
                    http_plugins.clone(),
                    websocket_plugins.clone(),
                )
            });

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
    state: Arc<State>,
    http_plugins: Arc<Vec<Box<dyn Plugin>>>,
    websocket_plugins: Arc<Vec<Box<dyn Plugin>>>,
) -> Result<ProxyResponse, hyper::Error> {
    if let Some(route) = state.config.match_route(req.uri().path()) {
        if let Err(err) = execute_plugins(&mut req, &http_plugins) {
            match err {
                PluginError::Http(status, body) => {
                    let mut res = Response::new(body);
                    *res.status_mut() = status;
                    return Ok(res);
                }
            }
        }

        return match route.protocol {
            Protocol::Http => handle_http(req, route, state.clone()).await,
            Protocol::Websocket => {
                handle_websocket(req, route, state.clone(), websocket_plugins.clone()).await
            }
        };
    }

    let mut res = Response::new(full("Invalid path"));
    *res.status_mut() = StatusCode::UNPROCESSABLE_ENTITY;

    Ok(res)
}

async fn handle_http(
    req: Request<Incoming>,
    route: &Route,
    state: Arc<State>,
) -> Result<ProxyResponse, hyper::Error> {
    let dmtr_project_id = req
        .headers()
        .get(DMTR_PROJECT_ID)
        .map(|k| k.to_str().unwrap_or_default())
        .unwrap_or_default();

    state.metrics.count_http_total_request(dmtr_project_id);

    let stream = TcpStream::connect((route.host.clone(), route.port))
        .await
        .unwrap();

    let io = TokioIo::new(stream);

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
    route: &Route,
    state: Arc<State>,
    websocket_plugins: Arc<Vec<Box<dyn Plugin>>>,
) -> Result<ProxyResponse, hyper::Error> {
    let headers = req.headers();

    if req.method() != Method::GET
        || !headers
            .get(UPGRADE)
            .and_then(|h| h.to_str().ok())
            .map(|h| h.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false)
    {
        let mut res = Response::new(full("Invalid websocket request"));
        *res.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
        return Ok(res);
    }

    let upgrade = HeaderValue::from_static("Upgrade");
    let websocket = HeaderValue::from_static("websocket");

    let key = headers.get(SEC_WEBSOCKET_KEY);
    let derived = key.map(|k| derive_accept_key(k.as_bytes()));
    let version = req.version();

    let host = route.host.clone();
    let port = route.port;

    let dmtr_project_id = req
        .headers()
        .get(DMTR_PROJECT_ID)
        .map(|k| k.to_str().unwrap_or_default())
        .unwrap_or_default()
        .to_string();

    tokio::task::spawn(async move {
        match hyper::upgrade::on(&mut req).await {
            Ok(upgraded) => {
                let upgraded = TokioIo::new(upgraded);
                dbg!(&upgraded);

                let client_stream =
                    WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await;
                let (client_outgoing, client_incoming) = client_stream.split();

                let url_result = Url::parse(&format!("ws://{}:{}", host, port));
                if let Err(err) = url_result {
                    error!(
                        error = err.to_string(),
                        "host and port invalid to mount the url on route config"
                    );
                    return;
                }
                let url = url_result.unwrap();

                let connection_result = connect_async(url).await;
                if let Err(err) = connection_result {
                    error!(error = err.to_string(), "fail to connect to the host");
                    return;
                }
                let (host_stream, _) = connection_result.unwrap();
                let (host_outgoing, host_incoming) = host_stream.split();

                let client_in = client_incoming
                    .inspect_ok(|_| {
                        let _result = execute_plugins(&mut req, &websocket_plugins);
                        state.metrics.count_ws_total_frame(&dmtr_project_id);
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
