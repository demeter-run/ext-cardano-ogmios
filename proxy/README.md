# Ext Cardano Ogmios Proxy

The proxy will manage the connection to the Ogmios, when a user makes a request, the proxy will decide which Ogmios instance will be requested using the hostname.

An example about how the proxy will decide which instance will be requested.

| Host                         | Instance         |
| ---------------------------- | ---------------- |
| mainnet.ogmios-1.demeter.run | ogmios-mainnet-1 |


The proxy exposes metrics about HTTP requests and WebSocket frames.

## Environment

| Key             | Value          |
| --------------- | -------------- |
| PROXY_ADDR      | "0.0.0.0:8100" |
| PROMETHEUS_ADDR | "0.0.0.0:5000" |
| OGMIOS_PORT     | -              |
| OGMIOS_DNS      | cluster dns suffix for ogmios services |
| HEALTH_NETWORK  | health upstream network, defaults to cardano-mainnet |
| SSL_CRT_PATH    | file.crt       |
| SSL_KEY_PATH    | file.key       |
| RPC_TELEMETRY_NETWORKS | networks whose client JSON-RPC messages are classified and counted: a comma list, or `*` for all; unset or empty disables it |


## Commands

Execute the proxy

```bash
cargo run
```

## Metrics

to collect metrics for Prometheus, an HTTP API will enable the route /metrics.

```
/metrics
```

With `RPC_TELEMETRY_NETWORKS` set, the proxy classifies every client JSON-RPC
message (WebSocket text and binary frames, HTTP POST bodies) into a bounded
method and shape vocabulary (`src/rpc/`) and counts it:

| Metric | Labels |
| ------ | ------ |
| `ogmios_proxy_rpc_requests_total` | `network`, `version`, `tier`, `transport`, `method`, `shape`, `heavy` |
| `ogmios_proxy_rpc_consumer_requests_total` | `consumer`, `network`, `version`, `family` |
| `ogmios_proxy_rpc_consumer_heavy_requests_total` | `consumer`, `network`, `version`, `method`, `shape` (heavy classes only) |

Forwarding is unchanged: an HTTP body is buffered up to 64 KiB to classify it
and then forwarded byte for byte. `cargo bench -p proxy` measures the
classifier.
