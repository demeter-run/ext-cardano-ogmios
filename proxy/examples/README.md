# Proxy Example

execute docker compose up to run proxy container and a mock api, after that it's possible to request /headers of the mock api


in the proxy.toml file some plugins is enabled.

## Metrics

To access prometheus metrics, open the link below

```
http://0.0.0.0:5000/metrics
```

## Http

the key on the host will be extract by the plugin and inject in the header.

```bash
curl http://dmtr_kupo1d988zmt0g4skjdt6xdxkg5zp23f55ttpxfmsemy6fa.0.0.0.0.nip.io/headers
```

## Websocket

Below a script to access the mock api using websocket.

```nodejs
const WebSocket = require("ws");

const ws = new WebSocket("ws://0.0.0.0:80", {
  headers: {
    "dmtr-api-key": `dmtr_kupo1d988zmt0g4skjdt6xdxkg5zp23f55ttpxfmsemy6fa`,
  },
});
let interval = 0;

ws.on("open", () => {
  console.log("Connected to WebSocket server");

  interval = setInterval(() => {
    ws.send(`Hello, WebSocket server! ${Math.random()}`);
  }, 10);
});

ws.on("message", (message) => {
  console.log(`Received message from server: ${message}`);
});

ws.on("close", () => {
  console.log("Connection to WebSocket server closed");
  clearInterval(interval);
});

ws.on("error", (err) => {
  console.error(err);
});

```