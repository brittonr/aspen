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

- [x] 3.1 Add `iroh::Endpoint` field to `Socks5Server` struct (shared via `Arc`)
- [x] 3.2 Replace the tunnel placeholder in `handle_connection()`: parse `endpoint_id` to `EndpointId`, build `EndpointAddr`, call `open_tunnel()`
- [x] 3.3 After QUIC connect: `open_tunnel` sends `u16` port, then `send_reply(REPLY_SUCCESS)` in handle_connection
- [x] 3.4 After success reply: `tokio::io::copy` bidirectional between TCP stream and QUIC streams
- [x] 3.5 Handle connect errors: invalid endpoint_id → HOST_UNREACHABLE, tunnel failure → HOST_UNREACHABLE
- [x] 3.6 Update `Socks5Server::new()` to take an `Arc<iroh::Endpoint>` parameter
- [x] 3.7 Update existing SOCKS5 integration tests to pass test endpoint, adjust assertions for tunnel-aware behavior

## 4. NetHandler Registration in Node

- [x] 4.1 `aspen-net` already an optional dep in root Cargo.toml; added `aspen-rpc-handlers/net` to `net` feature chain
- [x] 4.2 Registered via `submit_handler_factory!(NetHandlerFactory)` in `aspen-net/src/lib.rs`; inventory auto-collects when crate is linked
- [x] 4.3 Verified `cargo check --features net` compiles successfully
- [x] 4.4 Added `net` to `full-aspen-node` features and `full-aspen-node-plugins` cargoExtraArgs in flake.nix

## 5. Daemon Cluster Wiring

- [x] 5.1 Added `aspen-client` and `aspen-transport` as dependencies in `crates/aspen-net/Cargo.toml`
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

- [x] 6.1 Registered `TunnelAcceptor` in `setup/router.rs` behind `#[cfg(feature = "net")]` with NET_TUNNEL_ALPN
- [x] 6.2 Verified Router builder accepts the new protocol alongside existing ALPNs

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
