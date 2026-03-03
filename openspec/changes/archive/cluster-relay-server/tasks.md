## 1. Feature Flag and Dependencies

- [x] 1.1 Add `relay-server` feature flag to `aspen-cluster/Cargo.toml` gating `iroh-relay` with the `server` feature
- [x] 1.2 Add `relay-server` feature flag to the workspace root `Cargo.toml` that enables `aspen-cluster/relay-server`
- [x] 1.3 Verify `iroh-relay = { version = "0.95.1", features = ["server"] }` compiles with `cargo check --features relay-server`

## 2. Relay Server Configuration

- [x] 2.1 Add `RelayServerConfig` struct to `crates/aspen-cluster/src/config/iroh.rs` with fields: `enabled`, `bind_addr` (default `0.0.0.0`), `bind_port` (default `3340`), `key_cache_capacity` (default `1024`), `client_rate_limit` (optional)
- [x] 2.2 Add `relay_server` field to `IrohConfig` (optional, `None` by default)
- [x] 2.3 Add relay server fields to `IrohEndpointConfig` for passing resolved config to the endpoint manager
- [x] 2.4 Wire relay server config through the config loading pipeline (TOML deserialization, CLI args, env vars)

## 3. Relay Server Lifecycle

- [x] 3.1 Create `crates/aspen-cluster/src/relay_server.rs` module (gated behind `#[cfg(feature = "relay-server")]`)
- [x] 3.2 Implement `RelayServer` struct wrapping `iroh_relay::server::Server` with spawn, shutdown, and addr methods
- [x] 3.3 Implement `RelayServer::spawn()` that creates `ServerConfig` with `RelayConfig` (HTTP, no TLS), `AccessConfig::Everyone` initially, and configured bind address/port
- [x] 3.4 Implement `RelayServer::shutdown()` with graceful drain and bounded timeout
- [x] 3.5 Add `relay_server` field to `IrohEndpointManager` (optional, feature-gated)
- [x] 3.6 Add `spawn_relay_server()` method on `IrohEndpointManager` that starts the relay and stores the handle

## 4. Access Control

- [x] 4.1 Define an `AllowedEndpoints` trait or shared `Arc<RwLock<HashSet<EndpointId>>>` for the set of authorized endpoint IDs
- [x] 4.2 Implement `AccessConfig::Restricted` callback that checks the shared allowed-endpoints set
- [x] 4.3 Update `RelayServer::spawn()` to accept the allowed-endpoints set and wire it into the access config
- [x] 4.4 Add method to update the allowed-endpoints set when Raft membership changes

## 5. Node Metadata: Relay URL

- [x] 5.1 Add `relay_url: Option<String>` field to the Raft `Node` metadata type (in `aspen-raft` or `aspen-core` types)
- [x] 5.2 Populate `relay_url` when a node with `relay-server` enabled registers itself in the cluster
- [x] 5.3 Ensure `relay_url` is serialized/deserialized correctly in Raft membership propagation

## 6. Dynamic Relay Map Construction

- [x] 6.1 Implement `build_relay_map_from_membership()` function that reads all nodes' relay URLs from Raft membership and constructs an `iroh::RelayMap`
- [x] 6.2 Add a membership watcher (subscribe to Raft metrics) that rebuilds the relay map when membership changes
- [x] 6.3 Implement mechanism to update the iroh endpoint's relay configuration when the relay map changes (or document if iroh requires endpoint recreation)

## 7. Self-Hosted Relay Mode Integration

- [x] 7.1 Update `IrohEndpointManager::new()` to detect when `relay-server` is enabled and auto-set `RelayMode::Custom` with the initial relay map (own relay URL for first node)
- [x] 7.2 Handle the bootstrap chicken-and-egg: first node uses its own relay, subsequent nodes discover relays from cluster ticket or membership
- [x] 7.3 Ensure explicit `relay_mode` override in config takes precedence over auto-configuration

## 8. Node Startup Integration

- [x] 8.1 Update `aspen_node/main.rs` bootstrap sequence to spawn the relay server early (before iroh endpoint relay mode is finalized)
- [x] 8.2 Wire the relay server's bound address into the node metadata for Raft registration
- [x] 8.3 Wire the allowed-endpoints set into the Raft membership change callback
- [x] 8.4 Add relay server shutdown to the node's graceful shutdown sequence (before iroh endpoint close)

## 9. Tests

- [x] 9.1 Unit test: `RelayServerConfig` defaults and custom configuration
- [x] 9.2 Integration test: relay server spawns and accepts connections on configured port
- [x] 9.3 Integration test: access control denies unknown endpoint IDs
- [x] 9.4 Integration test: two nodes form a cluster, each discovers the other's relay URL in membership
- [x] 9.5 Integration test: node uses cluster relay for communication when direct connection is unavailable
- [x] 9.6 Test: relay server graceful shutdown drains connections

## 10. Documentation

- [x] 10.1 Document the `relay-server` feature flag in the crate-level docs and README
- [x] 10.2 Add relay server configuration section to the node configuration reference
- [x] 10.3 Document the port requirements (HTTP relay port, optional QUIC QAD port)
- [x] 10.4 Document the security model: iroh end-to-end encryption, access control, TLS as future enhancement
