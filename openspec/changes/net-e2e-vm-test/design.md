# Design: Net End-to-End VM Test

## Architecture

### Handler Registration

The `NetHandler<S>` already implements `RequestHandler` with `can_handle` matching `NetPublish`, `NetUnpublish`, `NetLookup`, `NetList`. It needs to be registered in the node's handler setup, gated behind `#[cfg(feature = "net")]`.

Registration site: `src/bin/aspen_node/setup/client.rs` — where other native handlers are created and pushed into the handler list. The handler takes an `Arc<ServiceRegistry<S>>` wrapping the node's KV store.

### Daemon Cluster Connection

The daemon currently takes a `cluster_ticket` string but never connects. The wiring:

1. Parse the cluster ticket to extract endpoint addresses
2. Create an iroh `Endpoint` (client-only, no server protocols needed)
3. Create an `AspenClient` (from `aspen-client`) that speaks the client RPC protocol
4. Wrap the client in a `ClientKvAdapter` that implements `KeyValueStore` by translating to RPCs
5. Pass the adapter to `ServiceRegistry` and `NameResolver`
6. Bind the SOCKS5 `TcpListener` and run `Socks5Server::run()`

### SOCKS5 Tunnel Completion

After the handshake resolves `(endpoint_id, remote_port)`, the proxy needs to:

1. Parse `endpoint_id` → `iroh::endpoint::EndpointAddr`
2. Call `iroh_endpoint.connect(addr, ALPN)` to open a QUIC stream to the target node
3. Send a connection header (target port) on the QUIC stream
4. Bidirectionally copy bytes between the TCP socket and the QUIC stream
5. Clean up on either side closing

The target node needs a protocol handler that accepts these tunnel connections — a `TunnelAcceptor` registered via the iroh Router that listens on a `NET_TUNNEL_ALPN`. When a tunnel request arrives, it connects to `localhost:{port}` and copies bytes.

### ALPN Protocol

New constant: `NET_TUNNEL_ALPN = b"/aspen/net-tunnel/0"` in `aspen-transport/src/lib.rs`.

Wire format on the QUIC stream:

- Client sends: `u16` (port, big-endian)
- Server reads port, connects to `127.0.0.1:{port}`, then bidirectional copy
- Either side closing the stream terminates the tunnel

### VM Test Design

Three-node Raft cluster. Node1 runs a Python HTTP server on port 8080. Node1 publishes the service. Node2 runs `aspen-net up` (the daemon with SOCKS5 proxy). A `curl` command on node2 routes through the SOCKS5 proxy to reach node1's HTTP server by name.

```
node2$ curl --socks5-hostname 127.0.0.1:1080 http://my-http.aspen:8080/
  → SOCKS5 proxy resolves "my-http.aspen" via KV lookup
  → Gets endpoint_id + port for node1
  → Opens iroh QUIC tunnel to node1
  → node1's TunnelAcceptor connects to localhost:8080
  → Response flows back through the tunnel
```

## Key Decisions

- **ClientKvAdapter vs direct RPC**: Use a thin adapter that implements `KeyValueStore` by calling RPC. This lets `ServiceRegistry` and `NameResolver` work unchanged — they're already generic over `KeyValueStore`.
- **TunnelAcceptor on every node**: Any node that publishes services needs to accept tunnel connections. Wire it into the node's iroh Router alongside RAFT_ALPN, CLIENT_ALPN, etc.
- **No auth in VM test**: The tunnel acceptor in the test trusts all connections from cluster peers. Real auth enforcement is out of scope.

## Files Changed

- `crates/aspen-transport/src/lib.rs` — add `NET_TUNNEL_ALPN`
- `crates/aspen-net/src/tunnel.rs` — new: tunnel acceptor + SOCKS5 tunnel wiring
- `crates/aspen-net/src/client_kv.rs` — new: `ClientKvAdapter` implementing `KeyValueStore` via RPC
- `crates/aspen-net/src/daemon.rs` — wire cluster connection, SOCKS5 server, tunnel acceptor
- `crates/aspen-net/src/socks5.rs` — replace tunnel placeholder with real iroh QUIC tunnel
- `crates/aspen-net/src/lib.rs` — add new modules
- `crates/aspen-net/Cargo.toml` — add `aspen-client` dep
- `src/bin/aspen_node/setup/client.rs` — register NetHandler (feature-gated)
- `Cargo.toml` — wire `net` feature through to rpc-handlers if needed
- `flake.nix` — add `net` to node build features, wire VM test
- `nix/tests/net-service-mesh.nix` — rewrite test script for real SOCKS5 routing
