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
- [x] 5.2 Daemon connects via `AspenClient::connect()` which parses the ticket and creates an iroh endpoint
- [x] 5.3 Endpoint created by AspenClient (discovery via ticket bootstrap peers)
- [x] 5.4 `ClientKvAdapter::new(client)` wraps the AspenClient for KV access
- [x] 5.5 `ServiceRegistry::new(kv_adapter)` and `NameResolver::new(registry)` built from the adapter
- [x] 5.6 `NetAuthenticator::permissive()` creates a self-signed NetAdmin token for trusted environments
- [x] 5.7 `Socks5Server::new(resolver, auth, endpoint, cancel)` with real iroh endpoint
- [x] 5.8 `TcpListener::bind(socks5_addr)` + spawn `socks5.run(listener)` as background task
- [x] 5.9 `TunnelAcceptor` registered via `Router::builder(endpoint).accept(NET_TUNNEL_ALPN, ...)`
- [x] 5.10 Auto-publish loop parses `--publish` specs, builds `ServiceEntry` with endpoint_id, calls `registry.publish()`
- [x] 5.11 Updated daemon test: invalid ticket returns ClusterConnect error (verifies real connection path)

## 6. TunnelAcceptor in Node

- [x] 6.1 Registered `TunnelAcceptor` in `setup/router.rs` behind `#[cfg(feature = "net")]` with NET_TUNNEL_ALPN
- [x] 6.2 Verified Router builder accepts the new protocol alongside existing ALPNs

## 7. NixOS VM Test

- [x] 7.1 Wire `nix/tests/net-service-mesh.nix` into `flake.nix` as `checks.x86_64-linux.net-service-mesh-test`
- [x] 7.2 Add `full-aspen-net-daemon` package to flake.nix via `fullBin { name = "aspen-net"; features = ["net"]; }`
- [x] 7.3 Test node has `aspenNetPackage`, `aspenCliPackage`, `python3`, `curl` in systemPackages
- [x] 7.4 Phase 1: Cluster bootstrap (wait_for_unit, wait_for_file ticket, cluster health, cluster init, KV plugin install)
- [x] 7.5 Phase 2: Python HTTP server on port 8080 serving `/tmp/www/test.txt`
- [x] 7.6 Phase 3: `aspen-cli net publish my-http` with endpoint_id, port 8080
- [x] 7.7 Phase 4 (simplified): Single-node test — verify `net services` lists `my-http` and `net lookup` resolves
- [x] 7.8 Phase 5: `aspen-net up --ticket ... --socks5-port 1080 --no-dns` as background process on same node
- [x] 7.9 Phase 6: `curl --socks5-hostname 127.0.0.1:1080 http://my-http.aspen:8080/test.txt` asserts content match
- [x] 7.10 Phase 7: Unpublish service, wait for cache expiry, verify SOCKS5 lookup fails
- [x] 7.11 Phase 8: Multi-service test — publish my-http and my-api, verify both in services list

## 8. Test Verification

- [x] 8.1 `cargo nextest run -p aspen-net` — 49 passed, 7 skipped
- [x] 8.2 `cargo nextest run` (full workspace) — 813 tests passed, 0 failed
- [x] 8.3 `nix eval --impure .#checks.x86_64-linux.net-service-mesh-test` evaluates to derivation ("set")
