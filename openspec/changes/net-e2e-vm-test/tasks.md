## 1. Transport Constant and Tunnel Protocol

- [x] 1.1 Add `NET_TUNNEL_ALPN = b"/aspen/net-tunnel/0"` to `crates/aspen-transport/src/constants.rs`
- [x] 1.2 Create `crates/aspen-net/src/tunnel.rs` with `TunnelAcceptor` struct: holds `CancellationToken`, `Arc<AtomicU32>` active count, max connections
- [x] 1.3 Implement `TunnelAcceptor::handle_stream()`: read `u16` port from QUIC stream, connect `TcpStream` to `127.0.0.1:{port}`, `tokio::io::copy` bidirectional, decrement active count on completion
- [x] 1.4 Implement `TunnelAcceptor` as iroh `ProtocolHandler`: accept incoming connections on `NET_TUNNEL_ALPN`, spawn tasks up to `MAX_SOCKS5_CONNECTIONS`
- [x] 1.5 Add tunnel unit tests + `open_tunnel()` client-side function
- [x] 1.6 Add `pub mod tunnel;` to `crates/aspen-net/src/lib.rs`

## 2. ClientKvAdapter

- [x] 2.1 Create `crates/aspen-net/src/client_kv.rs` with `ClientKvAdapter` struct wrapping `Arc<AspenClient>`
- [x] 2.2 Implement `KeyValueStore::read()`: send `ReadKey` RPC, parse `ReadResultResponse`
- [x] 2.3 Implement `KeyValueStore::write()`: send `WriteKey` RPC, parse `WriteResultResponse`
- [x] 2.4 Implement `KeyValueStore::delete()`: send `DeleteKey` RPC, parse `DeleteResultResponse`
- [x] 2.5 Implement `KeyValueStore::scan()`: send `ScanKeys` RPC, parse `ScanResultResponse`
- [x] 2.6 Add `aspen-client`, `aspen-transport`, `async-trait` to `crates/aspen-net/Cargo.toml`
- [x] 2.7 Add unit tests: verify RPC request construction for write, read, scan
- [x] 2.8 Add `pub mod client_kv;` to `crates/aspen-net/src/lib.rs`

## 3. SOCKS5 Tunnel Completion

- [ ] 3.1 Add `iroh::Endpoint` field to `Socks5Server` struct (shared via `Arc`)
- [ ] 3.2 Replace the tunnel placeholder in `handle_connection()`: parse `endpoint_id` string to `iroh::EndpointId`, build `EndpointAddr`, call `endpoint.connect(addr, NET_TUNNEL_ALPN)`
- [ ] 3.3 After QUIC connect: send `u16` port (big-endian) on the QUIC stream, then `send_reply(REPLY_SUCCESS)`
- [ ] 3.4 After success reply: `tokio::io::copy_bidirectional` between the TCP `stream` and QUIC `SendStream`/`RecvStream` (adapt to `AsyncRead`/`AsyncWrite`)
- [ ] 3.5 Handle connect errors: map iroh connection errors to SOCKS5 reply codes (`HOST_UNREACHABLE`, `CONNECTION_REFUSED`)
- [ ] 3.6 Update `Socks5Server::new()` to take an `Arc<iroh::Endpoint>` parameter
- [ ] 3.7 Update existing SOCKS5 integration tests to pass a mock/stub endpoint

## 4. NetHandler Registration in Node

- [ ] 4.1 Add `aspen-net` as optional dependency in root `Cargo.toml` [dependencies] (already present as `aspen-net = { workspace = true, optional = true }`)
- [ ] 4.2 In `src/bin/aspen_node/setup/client.rs`: add `#[cfg(feature = "net")]` block that creates `ServiceRegistry` from the node's KV store, wraps in `NetHandler`, pushes into handler list
- [ ] 4.3 Verify `cargo check --features net` compiles the handler registration
- [ ] 4.4 Add `net` feature to `full-aspen-node` and `full-aspen-node-plugins` feature lists in `flake.nix`

## 5. Daemon Cluster Wiring

- [ ] 5.1 Add `aspen-client` and `aspen-transport` as dependencies in `crates/aspen-net/Cargo.toml`
- [ ] 5.2 In `NetDaemon::start()`: parse `cluster_ticket` (extract peer addresses from the ticket format used by aspen-client)
- [ ] 5.3 Create an `iroh::Endpoint` (discovery disabled for daemon — addresses come from ticket)
- [ ] 5.4 Create `ClientKvAdapter` from the endpoint and peer addresses
- [ ] 5.5 Create `ServiceRegistry` and `NameResolver` from the adapter
- [ ] 5.6 Create `NetAuthenticator` (permissive for now — allows all `.aspen` domains)
- [ ] 5.7 Create `Socks5Server` with the endpoint, resolver, and authenticator
- [ ] 5.8 Bind `TcpListener` on `socks5_addr`, spawn `socks5_server.run(listener)` as background task
- [ ] 5.9 Optionally register `TunnelAcceptor` on the daemon's iroh endpoint (so daemon can also receive tunnels)
- [ ] 5.10 Wire auto-publish: parse `--publish` specs, call `registry.publish()` for each
- [ ] 5.11 Add daemon integration test: start daemon with mock cluster, verify SOCKS5 listener binds

## 6. TunnelAcceptor in Node

- [ ] 6.1 In the node's iroh Router setup: when `net` feature is enabled, register `TunnelAcceptor` to accept `NET_TUNNEL_ALPN` connections
- [ ] 6.2 Verify the Router builder accepts the new protocol alongside existing ALPNs (RAFT_ALPN, CLIENT_ALPN, etc.)

## 7. NixOS VM Test

- [ ] 7.1 Wire `nix/tests/net-service-mesh.nix` into `flake.nix` as `checks.x86_64-linux.net-service-mesh-test`
- [ ] 7.2 Add `aspen-net` binary package to flake.nix (built from `crates/aspen-net/bin/aspen-net.rs` with appropriate features)
- [ ] 7.3 Update `nix/tests/net-service-mesh.nix` node config: add `aspen-net` binary to `environment.systemPackages`
- [ ] 7.4 Update test: Phase 1 — cluster bootstrap using established pattern (wait_for_unit, wait_for_file, cluster init, wait_until_succeeds health check)
- [ ] 7.5 Update test: Phase 2 — start python3 HTTP server on node1 serving `/tmp/www/test.txt` on port 8080
- [ ] 7.6 Update test: Phase 3 — publish service: `aspen-cli net publish my-http --endpoint-id <id> --port 8080`
- [ ] 7.7 Update test: Phase 4 — verify `aspen-cli net services` lists `my-http` from both node1 and node2
- [ ] 7.8 Update test: Phase 5 — start `aspen-net up` on node2 as background process with `--socks5-port 1080 --no-dns`
- [ ] 7.9 Update test: Phase 6 — `curl --socks5-hostname 127.0.0.1:1080 http://my-http.aspen:8080/test.txt` on node2, assert response matches expected content
- [ ] 7.10 Update test: Phase 7 — unpublish service, verify SOCKS5 lookup fails (curl returns error)
- [ ] 7.11 Update test: Phase 8 — publish second service on node3, verify reachable through SOCKS5 from node2

## 8. Test Verification

- [ ] 8.1 Run `cargo nextest run -p aspen-net` — all existing + new unit tests pass
- [ ] 8.2 Run `cargo nextest run -p aspen` — handler registration compiles, no regressions
- [ ] 8.3 Verify `nix build .#checks.x86_64-linux.net-service-mesh-test --impure` evaluates (derivation builds)
