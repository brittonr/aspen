## 1. Crate Scaffold and Feature Wiring

- [ ] 1.1 Create `crates/aspen-net/Cargo.toml` with dependencies: `aspen-core`, `aspen-constants`, `aspen-client-api`, `aspen-proxy` (for DownstreamProxy re-export), `iroh`, `iroh-proxy-utils`, `tokio`, `serde`, `serde_json`, `snafu`, `tracing`
- [ ] 1.2 Create `crates/aspen-net/src/lib.rs` with module declarations: `registry`, `resolver`, `socks5`, `forward`, `auth`, `dns`, `daemon`, `types`, `constants`, `verified`
- [ ] 1.3 Add `net` feature flag to workspace root `Cargo.toml` that enables `aspen-net` and wires through `aspen-rpc-handlers/net`
- [ ] 1.4 Add `aspen-net` to workspace `members` list
- [ ] 1.5 Verify `cargo check -p aspen-net` compiles with empty module stubs

## 2. Types and Constants

- [ ] 2.1 Create `crates/aspen-net/src/types.rs` with `ServiceEntry` struct: `name: String`, `endpoint_id: String`, `port: u16`, `proto: String`, `tags: Vec<String>`, `hostname: Option<String>`, `published_at_ms: u64`. Derive `Serialize`, `Deserialize`, `Clone`, `Debug`.
- [ ] 2.2 Add `NodeEntry` struct: `endpoint_id: String`, `hostname: String`, `tags: Vec<String>`, `services: Vec<String>`, `last_seen_ms: u64`
- [ ] 2.3 Add `DnsOverride` struct: `hostname: String`, `address: String`
- [ ] 2.4 Create `crates/aspen-net/src/constants.rs` with Tiger Style bounds:
  - `MAX_NET_SERVICES: u32 = 10_000`
  - `MAX_NET_TAGS_PER_SERVICE: u32 = 32`
  - `MAX_NET_SERVICE_NAME_LEN: u32 = 253`
  - `MAX_NET_DNS_LOOPBACK_ADDRS: u32 = 254`
  - `MAX_SOCKS5_CONNECTIONS: u32 = 1_000`
  - `SOCKS5_HANDSHAKE_TIMEOUT_SECS: u64 = 10`
  - `SOCKS5_IDLE_TIMEOUT_SECS: u64 = 300`
  - `NET_DNS_TTL_SECS: u32 = 5`
  - `NET_REGISTRY_POLL_INTERVAL_SECS: u64 = 10`
  - `NET_REVOCATION_POLL_INTERVAL_SECS: u64 = 60`
  - `NET_SHUTDOWN_TIMEOUT_SECS: u64 = 30`
- [ ] 2.5 Add KV prefix constants: `NET_SVC_PREFIX = "/_sys/net/svc/"`, `NET_NODE_PREFIX = "/_sys/net/node/"`, `NET_DNS_PREFIX = "/_sys/net/dns/"`
- [ ] 2.8 Add compile-time assertions for all constants (positive, within bounds)

## 3. Service Registry

- [ ] 3.1 Create `crates/aspen-net/src/registry.rs` with `ServiceRegistry` struct holding a KV store reference (generic over `KeyValueStore` trait)
- [ ] 3.2 Implement `publish(&self, entry: ServiceEntry) -> Result<()>`: validate name (regex `^[a-z0-9][a-z0-9.-]{0,252}$`), validate tag count, serialize to JSON, write to `/_sys/net/svc/{name}`, update node entry
- [ ] 3.3 Implement `unpublish(&self, name: &str) -> Result<()>`: delete `/_sys/net/svc/{name}`, update node entry
- [ ] 3.4 Implement `lookup(&self, name: &str) -> Result<Option<ServiceEntry>>`: read `/_sys/net/svc/{name}`, deserialize
- [ ] 3.5 Implement `list(&self, tag_filter: Option<&str>) -> Result<Vec<ServiceEntry>>`: scan `/_sys/net/svc/` prefix, optionally filter by tag
- [ ] 3.6 Implement `list_nodes(&self) -> Result<Vec<NodeEntry>>`: scan `/_sys/net/node/` prefix
- [ ] 3.7 Add service name validation function in `verified/`: `is_valid_service_name(name: &str) -> bool`
- [ ] 3.8 Unit tests: publish, lookup, unpublish, list, list-by-tag, name validation, duplicate name overwrite, resource bounds

## 4. Name Resolver

- [ ] 4.1 Create `crates/aspen-net/src/resolver.rs` with `NameResolver` struct holding a `ServiceRegistry` reference and a local cache (`HashMap<String, ServiceEntry>`)
- [ ] 4.2 Implement `resolve(&self, name: &str) -> Result<Option<(EndpointId, u16)>>`: strip `.aspen` suffix, lookup in cache, fallback to registry, parse endpoint_id string to `iroh::EndpointId`
- [ ] 4.3 Implement `refresh_cache(&self) -> Result<()>`: list all services from registry, replace local cache
- [ ] 4.4 Add cache TTL: entries expire after `NET_REGISTRY_POLL_INTERVAL_SECS`, trigger refresh on next resolve
- [ ] 4.5 Unit tests: resolve known service, resolve unknown, cache hit, cache miss triggers refresh, `.aspen` suffix stripping

## 5. Capability Token Integration (aspen-auth)

- [ ] 5.1 Add `NetConnect { service_prefix: String }` variant to `Capability` enum in `crates/aspen-auth/src/capability.rs`
- [ ] 5.2 Add `NetPublish { service_prefix: String }` variant to `Capability` enum
- [ ] 5.3 Add `NetAdmin` variant to `Capability` enum
- [ ] 5.4 Add `NetConnect { service: String, port: u16 }` variant to `Operation` enum
- [ ] 5.5 Add `NetPublish { service: String }` variant to `Operation` enum
- [ ] 5.6 Add `NetUnpublish { service: String }` variant to `Operation` enum
- [ ] 5.7 Add `NetAdmin { action: String }` variant to `Operation` enum
- [ ] 5.8 Implement `authorizes()` match arms in `Capability`:
  - `NetConnect { service_prefix }` authorizes `Operation::NetConnect` when `service.starts_with(service_prefix)`
  - `NetPublish { service_prefix }` authorizes `Operation::NetPublish` / `NetUnpublish` when `service.starts_with(service_prefix)`
  - `NetAdmin` authorizes all `Net*` operations
- [ ] 5.9 Implement `contains()` match arms for delegation:
  - `NetConnect` contains `NetConnect` with narrower prefix
  - `NetPublish` contains `NetPublish` with narrower prefix
  - `NetAdmin` contains all `NetConnect`, `NetPublish`, `NetAdmin`
  - `NetConnect` does NOT contain `NetPublish` (different capability type)
- [ ] 5.10 Add `Display` impl for new `Operation` variants
- [ ] 5.11 Create `crates/aspen-net/src/auth.rs` with `NetAuthenticator` struct holding `TokenVerifier` and `CapabilityToken`:
  - `check_connect(&self, service: &str, port: u16) -> Result<()>`: verify token + authorize NetConnect
  - `check_publish(&self, service: &str) -> Result<()>`: verify token + authorize NetPublish
  - `is_token_valid(&self) -> bool`: check expiration without full verification
- [ ] 5.12 Add periodic revocation list refresh task (polls `KeyValueRevocationStore` every `NET_REVOCATION_POLL_INTERVAL_SECS`)
- [ ] 5.13 Unit tests in `aspen-auth`: NetConnect/NetPublish/NetAdmin authorization, prefix matching, delegation containment, NetAdmin contains all
- [ ] 5.14 Unit tests in `aspen-net`: NetAuthenticator token verification, expired token rejection, connect/publish checks

## 6. SOCKS5 Proxy Server

- [ ] 6.1 Create `crates/aspen-net/src/socks5.rs` with `Socks5Server` struct holding: `DownstreamProxy`, `NameResolver`, `NetAuthenticator`, bind address, active connection counter (`Arc<AtomicU32>`)
- [ ] 6.2 Implement SOCKS5 greeting/auth negotiation: read version (0x05), method count, methods. Reply with no-auth (0x00). Reject if version != 5.
- [ ] 6.3 Implement SOCKS5 CONNECT request parsing: read version, cmd (must be 0x01 = CONNECT), reserved, address type. Support only type 0x03 (domain name). Parse domain name and port.
- [ ] 6.4 Implement name resolution: strip `.aspen` suffix from domain, call `resolver.resolve()`. Return SOCKS5 error 0x04 (host unreachable) if not found.
- [ ] 6.5 Implement token authorization check: call `authenticator.check_connect(service_name, port)`. Return SOCKS5 error 0x05 (connection refused) if token doesn't grant `NetConnect` for this service.
- [ ] 6.6 Implement tunnel creation: call `downstream_proxy.create_tunnel(EndpointAuthority { endpoint_id, authority: Authority::new("localhost", port) })`. Return SOCKS5 error 0x05 on tunnel failure.
- [ ] 6.7 Implement SOCKS5 success response: send version (0x05), reply (0x00 = success), reserved (0x00), addr type (0x01), bind addr (0.0.0.0), bind port (0). Then bidirectionally copy bytes between TCP stream and tunnel streams.
- [ ] 6.8 Implement connection limit enforcement: increment counter on accept, decrement on close. Reject with SOCKS5 error 0x01 (general failure) if at `MAX_SOCKS5_CONNECTIONS`.
- [ ] 6.9 Implement handshake timeout: wrap greeting + request parsing in `tokio::time::timeout(SOCKS5_HANDSHAKE_TIMEOUT_SECS)`.
- [ ] 6.10 Implement `Socks5Server::run(&self, listener: TcpListener)` main accept loop with per-connection task spawning and cancellation token for shutdown.
- [ ] 6.11 Unit tests: greeting parsing, CONNECT parsing, domain extraction. Integration tests: tunnel through SOCKS5 to a local TCP echo server via loopback iroh endpoints.

## 7. Port Forwarding

- [ ] 7.1 Create `crates/aspen-net/src/forward.rs` with `PortForward` struct wrapping `DownstreamProxy`
- [ ] 7.2 Implement `PortForward::start(local_addr: SocketAddr, service_name: &str, remote_port: Option<u16>, resolver: &NameResolver, auth: &NetAuthenticator) -> Result<()>`:
  - Resolve service name
  - Token authorization check (`auth.check_connect(service_name, port)`)
  - Create `DownstreamProxy::new(endpoint, pool_opts)`
  - Build `ProxyMode::Tcp(EndpointAuthority { endpoint_id, authority: Authority::new("localhost", port) })`
  - Bind TCP listener on `local_addr`
  - Call `proxy.forward_tcp_listener(listener, mode)`
- [ ] 7.3 Parse forward spec string: `[bind_addr:]local_port:service[:remote_port]` → `(SocketAddr, String, Option<u16>)`
- [ ] 7.4 Unit tests: spec string parsing. Integration tests: forward local port, connect through it.

## 8. MagicDNS Stub Resolver

- [ ] 8.1 Create `crates/aspen-net/src/dns.rs` with `MagicDnsServer` struct holding: `NameResolver`, address map (`HashMap<String, Ipv4Addr>`), next loopback counter, upstream resolver address
- [ ] 8.2 Add `hickory-dns` (or `hickory-server`) dependency to `Cargo.toml` (feature-gated behind `dns`)
- [ ] 8.3 Implement DNS request handler: parse query, check if domain ends in `.aspen`, if yes → resolve via registry, if no → forward to upstream
- [ ] 8.4 Implement loopback address allocation: assign `127.0.0.{2..255}` to services, LRU eviction when full, consistent assignment for same service
- [ ] 8.5 Implement DNS response construction: A record with allocated loopback IP, TTL = `NET_DNS_TTL_SECS`
- [ ] 8.6 Implement NXDOMAIN response for unknown `.aspen` services
- [ ] 8.7 Implement AAAA query handling: return NOERROR with empty answer for `.aspen` domains
- [ ] 8.8 Implement DNS override lookup: check `/_sys/net/dns/{hostname}` before service registry
- [ ] 8.9 Implement `MagicDnsServer::run(&self, bind_addr: SocketAddr)` with UDP socket and per-query handling
- [ ] 8.10 Unit tests: `.aspen` resolution, NXDOMAIN, upstream forwarding, loopback allocation, override precedence

## 9. Client API RPC Types

- [ ] 9.1 Add `NetPublish(NetPublishRequest)` variant to `ClientRpcRequest` in `aspen-client-api` (feature-gated behind `net`)
- [ ] 9.2 Add `NetUnpublish(NetUnpublishRequest)` variant
- [ ] 9.3 Add `NetLookup(NetLookupRequest)` variant
- [ ] 9.4 Add `NetList(NetListRequest)` variant
- [ ] 9.5 Add `NetDnsSet(NetDnsSetRequest)` variant
- [ ] 9.6 Add `NetDnsList` variant
- [ ] 9.7 Add corresponding response variants to `ClientRpcResponse`
- [ ] 9.8 Define request/response structs with serde derives in `aspen-client-api/src/net.rs` (feature-gated)
- [ ] 9.9 Update `variant_name()`, `domain()`, `to_operation()` for all new variants
- [ ] 9.10 Add to HANDLES list and update handles_count test
- [ ] 9.11 Verify postcard discriminant stability: add golden-file tests for new variants (they must be added AFTER all non-gated variants, before other feature-gated sections, or at the end of the feature-gated block)

## 10. RPC Handler

- [ ] 10.1 Create `crates/aspen-net/src/handler.rs` (or add to aspen-rpc-handlers) implementing `ServiceExecutor` for net operations
- [ ] 10.2 Implement dispatch for: NetPublish, NetUnpublish, NetLookup, NetList, NetDnsSet, NetDnsList
- [ ] 10.3 Each handler: deserialize request, call `ServiceRegistry` method, serialize response. Token auth is enforced at the daemon/CLI level, not the RPC handler level (RPC handlers use cluster cookie auth like all other handlers).
- [ ] 10.4 Register net handler in `aspen-rpc-handlers` handler registry (feature-gated behind `net`)
- [ ] 10.5 Wire UpstreamProxy into node's iroh router when `net` feature is enabled (register on `iroh-http-proxy/1` ALPN)
- [ ] 10.6 Integration test: publish service via RPC, lookup via RPC, verify round-trip

## 11. Daemon Orchestration

- [ ] 11.1 Create `crates/aspen-net/src/daemon.rs` with `NetDaemon` struct holding: iroh `Endpoint`, `DownstreamProxy`, `NameResolver`, `NetAuthenticator`, `Socks5Server`, `MagicDnsServer` (optional), `CancellationToken`
- [ ] 11.2 Implement `NetDaemon::start(config: DaemonConfig) -> Result<Self>`:
  - Create iroh endpoint
  - Connect to cluster via ticket (create a client that can make RPCs)
  - Initialize `ServiceRegistry` (via client RPCs)
  - Initialize `NameResolver` with registry
  - Initialize `NetAuthenticator` with token and verifier
  - Start SOCKS5 server on configured port
  - Start MagicDNS if enabled
  - Auto-publish configured local services
  - Spawn registry watcher task (polls every `NET_REGISTRY_POLL_INTERVAL_SECS`)
- [ ] 11.3 Implement `NetDaemon::shutdown(&self)` with graceful drain
- [ ] 11.4 Create `crates/aspen-net/bin/aspen-net.rs` binary with clap CLI:
  - Subcommands: `up`, `down`, `forward`, `publish`, `unpublish`, `services`, `peers`, `status`, `dns` (set/list)
- [ ] 11.5 Implement signal handling (SIGTERM, SIGINT → graceful shutdown)

## 12. CLI Integration

- [ ] 12.1 Add `net` subcommand group to `aspen-cli` with subcommands: `publish`, `unpublish`, `services`, `peers`, `dns` (set/list)
- [ ] 12.2 Implement `aspen-cli net publish <name> --port <port> [--proto tcp] [--tag <tag>]...`: sends `NetPublish` RPC (requires token with `NetPublish` capability)
- [ ] 12.3 Implement `aspen-cli net unpublish <name>`: sends `NetUnpublish` RPC
- [ ] 12.4 Implement `aspen-cli net services [--tag <tag>]`: sends `NetList` RPC, formats table output
- [ ] 12.5 Implement `aspen-cli net peers`: sends `NetList` + groups by endpoint, formats table
- [ ] 12.6 Implement `aspen-cli net dns set/list`: sends appropriate DNS RPCs (requires token with `NetAdmin` capability)
- [ ] 12.7 Note: `aspen net up/down/forward/proxy` live in the standalone `aspen-net` binary, not `aspen-cli`, because they need a local iroh endpoint (not just RPC calls)

## 13. Node-Side UpstreamProxy Integration

- [ ] 13.1 In aspen-node startup (when `net` feature enabled): create `AspenUpstreamProxy` and register on router with `iroh-http-proxy/1` ALPN
- [ ] 13.2 Use `AspenAuthHandler::with_trusted_peers()` for auth (same trusted-peers set as existing proxy, updated from Raft membership)
- [ ] 13.3 Verify: DownstreamProxy on daemon side can create_tunnel() to UpstreamProxy on node side

## 14. Tests

- [ ] 14.1 Unit tests for `verified/` pure functions (service name validation)
- [ ] 14.2 Unit tests in `aspen-auth`: NetConnect/NetPublish/NetAdmin capability authorization, prefix matching, delegation containment chains
- [ ] 14.3 Integration test: full SOCKS5 tunnel — daemon with valid token connects to cluster, publishes service, resolves name, tunnels TCP through iroh to a local TCP echo server
- [ ] 14.4 Integration test: port forward — forward local port to remote service, send data, receive response
- [ ] 14.5 Integration test: token deny — daemon with `NetConnect { service_prefix: "allowed/" }` token, verify SOCKS5 returns connection refused for `denied/mydb`
- [ ] 14.6 Integration test: expired token — daemon with expired token, verify new connections are rejected but existing tunnels continue
- [ ] 14.7 Integration test: service publish/unpublish lifecycle — publish, verify lookup, unpublish, verify gone
- [ ] 14.8 Integration test: delegation chain — root token delegates to team token with narrower scope, verify attenuation works
- [ ] 14.9 NixOS VM integration test: multi-node cluster with daemon, publish postgres on node A, forward from node B, run SQL query through tunnel

## 15. Documentation

- [ ] 15.1 Crate-level rustdoc for `aspen-net` with architecture overview and usage examples
- [ ] 15.2 Document `net` feature flag in workspace README and `AGENTS.md`
- [ ] 15.3 Document KV schema (`/_sys/net/` prefix) in design doc or README
- [ ] 15.4 Document CLI commands with examples in `aspen-net --help` and `aspen-cli net --help`
- [ ] 15.5 Add `aspen-net` to the "Key Modules" section in `AGENTS.md`
