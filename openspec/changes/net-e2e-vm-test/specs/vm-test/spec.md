# Spec: Net E2E VM Test

## Components

### NET_TUNNEL_ALPN

- Constant `b"/aspen/net-tunnel/0"` in aspen-transport
- Used by both SOCKS5 proxy (client side) and TunnelAcceptor (server side)

### TunnelAcceptor

- Iroh protocol handler accepting connections on NET_TUNNEL_ALPN
- Reads u16 port from QUIC stream, connects to 127.0.0.1:{port}, copies bytes bidirectionally
- Registered on every node that has `net` feature enabled
- Bounded: MAX_SOCKS5_CONNECTIONS concurrent tunnels

### ClientKvAdapter

- Implements `KeyValueStore` trait by delegating to `AspenClient` RPC calls
- Methods: read → KvGet, write → KvSet, delete → KvDelete, scan → KvScan
- Used by daemon-side ServiceRegistry and NameResolver

### SOCKS5 Tunnel (replacing placeholder)

- After handshake resolves (endpoint_id, port):
  1. Parse endpoint_id to EndpointAddr
  2. endpoint.connect(addr, NET_TUNNEL_ALPN)
  3. Send u16 port on QUIC stream
  4. tokio::io::copy_bidirectional between TCP stream and QUIC stream
- On error: send appropriate SOCKS5 reply code, close

### NetHandler Registration

- In setup/client.rs, when `net` feature enabled:
  - Create ServiceRegistry wrapping the node's KV store
  - Create NetHandler wrapping the registry
  - Push into handler list

### Daemon Wiring

- Parse cluster ticket → create iroh Endpoint → connect as client
- Create ClientKvAdapter from the client connection
- Create ServiceRegistry + NameResolver from the adapter
- Bind TcpListener on socks5_addr
- Start Socks5Server::run() in background task
- Start TunnelAcceptor on the iroh endpoint (so this daemon node can also receive tunnels)

## VM Test Scenario

### Setup

- 3-node Raft cluster with KV plugin
- Node1: python3 HTTP server on port 8080 serving a known file
- Node1: `aspen-cli net publish my-http --endpoint-id <node1_id> --port 8080 --proto tcp`
- Node2: `aspen-net up --ticket <ticket> --token <token> --socks5-port 1080 --no-dns`

### Assertions

1. Service published: `aspen-cli net services` lists `my-http`
2. Cross-node lookup: `aspen-cli net lookup my-http` from node2 returns node1's endpoint
3. SOCKS5 routing: `curl --socks5-hostname 127.0.0.1:1080 http://my-http.aspen:8080/test.txt` returns expected content
4. Unpublish: after `aspen-cli net unpublish my-http`, SOCKS5 connection to `my-http.aspen` fails
5. Multi-service: publish a second service, verify both resolve correctly
