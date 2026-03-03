## Context

Aspen's networking stack already includes two layers relevant to this change:

1. **iroh-proxy-utils** (vendored dependency): Provides `DownstreamProxy` (TCP listener → QUIC tunnel) and `UpstreamProxy` (iroh ProtocolHandler → TCP origin). Supports CONNECT tunneling, HTTP forward/reverse proxy, WebSocket upgrades, connection pooling, pluggable auth (`AuthHandler` trait), and pluggable routing (`RequestHandler` trait). Uses ALPN `iroh-http-proxy/1`.

2. **aspen-proxy**: Wraps iroh-proxy-utils with `AspenAuthHandler` (cluster cookie + trusted-peers allowlist). Re-exports `DownstreamProxy`, `ProxyMode`, `PoolOpts`, etc.

The data plane is fully built. What's missing is the control plane: naming services, discovering them, resolving names to iroh endpoints, enforcing access policies, and presenting this to users through CLI and local proxies.

All Raft KV operations use the existing `KeyValueStore` trait — the same pattern used by coordination primitives, forge, hooks, observability, and every other Aspen subsystem.

## Goals / Non-Goals

**Goals:**

- Named service registry backed by Raft consensus
- SOCKS5 proxy that resolves `*.aspen` names and tunnels through iroh
- TCP port forwarding from local port to named remote service
- Authorization via existing UCAN capability tokens (no new ACL system)
- DNS via existing `aspen-dns` crate for `*.aspen` domain resolution
- Daemon mode orchestrating all components
- CLI UX comparable to `tailscale up` / `wg-quick`

**Non-Goals:**

- TUN/TAP virtual network interfaces (no kernel involvement)
- IP address allocation (iroh endpoint IDs are the identity)
- UDP proxying (TCP-only in initial implementation; QUIC datagrams later)
- Subnet routing or exit node functionality
- Custom ALPN protocol (reuse existing `iroh-http-proxy/1`)
- Service health checking or load balancing (future enhancement)
- mTLS between services (iroh QUIC is already end-to-end encrypted)

## Decisions

### 1. Build on iroh-proxy-utils, not a new data plane

**Decision**: All tunneling uses `DownstreamProxy::create_tunnel()` and `DownstreamProxy::forward_tcp_listener()` from iroh-proxy-utils. No new QUIC stream handling, no new ALPN.

**Rationale**: iroh-proxy-utils already handles TCP↔QUIC bridging with connection pooling, metrics, auth, and error handling. Building a parallel data plane would duplicate ~2,000 lines of tested code. The CONNECT tunnel gives us raw bidirectional byte streams — sufficient for any TCP protocol.

**Trade-off**: We're constrained to TCP protocols. UDP (DNS, game servers, VoIP) won't work through the proxy. This is acceptable for the initial implementation; a QUIC datagram path can be added later for UDP.

### 2. Service names, not IP addresses

**Decision**: Services are identified by name (e.g., `mydb`, `web-frontend`). No IP addresses are allocated. The registry maps `name → (endpoint_id, port, proto, tags)`.

**Rationale**: iroh endpoint IDs are the identity layer. Allocating IP addresses would duplicate identity, require a consensus-backed allocator, and add complexity for no benefit. Applications connect by name; the proxy resolves to endpoint + port internally.

**Alternative considered**: Crypto-derived IPv6 from endpoint ID (like cjdns) — rejected because it still requires TUN and kernel routing, defeating the userspace-only goal.

### 3. SOCKS5 as the primary transparent proxy

**Decision**: Implement a SOCKS5 server (RFC 1928) that resolves `*.aspen` names from the service registry and creates CONNECT tunnels via `DownstreamProxy`.

**Rationale**: SOCKS5 is universally supported — `curl --socks5`, `ssh ProxyCommand`, `proxychains`, browser proxy settings, Docker `--network`. It handles arbitrary TCP protocols (not just HTTP). Implementation is ~200 lines for the handshake plus resolution logic. The actual tunneling delegates to `DownstreamProxy::create_tunnel()`.

**Alternative considered**: HTTP CONNECT proxy only — rejected because many TCP applications (databases, SSH, custom protocols) don't speak HTTP. SOCKS5 is protocol-agnostic.

### 4. UpstreamProxy on every node with service publishing

**Decision**: Every aspen-node that enables the `net` feature runs an `UpstreamProxy` as an iroh ProtocolHandler. Services are "published" by registering in Raft KV which node hosts them and on which local port.

**Rationale**: The upstream proxy accepts tunneled connections from any authorized peer and forwards to `localhost:{port}`. This is the same model as Tailscale's "funnel" but without the IP layer. The upstream proxy is already feature-complete in iroh-proxy-utils; we just register it on the node's iroh router.

### 5. Raft KV for all control plane state

**Decision**: Service registry, DNS overrides, and node metadata all stored in Raft KV under `/_sys/net/` prefix. Authorization is token-based (not stored in KV).

**Rationale**: Consistent with every other Aspen subsystem. Raft replication ensures all nodes see the same service registry. Authorization is decoupled from KV — tokens are verified locally using cryptographic signatures.

**KV Schema:**

```
/_sys/net/svc/{name}              → ServiceEntry (JSON)
/_sys/net/node/{endpoint_id_hex}  → NodeEntry (JSON)
dns:{name}.aspen:{type}           → DnsRecord (via aspen-dns DnsStore)
```

Note: No `/_sys/net/acl/` prefix — authorization is handled by capability tokens, not centralized rules.

### 6. UCAN capability tokens for authorization (not ACLs)

**Decision**: Use Aspen's existing UCAN-inspired capability tokens (`aspen-auth`) for authorization. Add new `Capability` variants (`NetConnect`, `NetPublish`, `NetAdmin`) and corresponding `Operation` variants. The daemon holds a capability token provided at startup and verifies it locally at connect/publish time.

**Rationale**: Aspen already has a complete capability token system with Ed25519 signing, delegation chains, prefix-based scoping, revocation, and offline verification. Building a separate ACL system would:

- Duplicate the authorization infrastructure
- Require Raft reads for every connection (ACL rules in KV)
- Miss the killer feature: delegation chains (admin → team → CI with attenuated access)
- Be less iroh-native (centralized vs. decentralized auth)

Capability tokens are verified locally with zero I/O — pure cryptographic verification. This is critical for a proxy that may handle hundreds of connections per second.

**New Capability Variants:**

```rust
/// Connect to named services through the mesh.
NetConnect { service_prefix: String }

/// Publish/unpublish services in the registry.
NetPublish { service_prefix: String }

/// Full net admin (manage registry, DNS overrides).
NetAdmin
```

**Delegation example:**

```
Root token: [NetAdmin + NetConnect { "*" } + Delegate]
  → Team token: [NetConnect { "prod/" } + NetPublish { "prod/" } + Delegate]
    → CI token: [NetConnect { "prod/staging-db" } + NetPublish { "prod/ci-*" }]
       (no Delegate — cannot create child tokens)
```

**Alternative considered**: Tag-based ACLs stored in Raft KV — rejected because it duplicates `aspen-auth`, requires consensus reads per connection, doesn't support delegation, and is less aligned with Aspen's decentralized philosophy.

### 7. DNS via existing aspen-dns crate

**Decision**: Use the existing `aspen-dns` sibling repo (`../aspen-dns/`) for all DNS functionality. When a service is published, auto-create DNS records (A, SRV) in the `aspen` zone via `DnsStore`. The existing `DnsProtocolServer` (hickory-server, UDP/TCP :5353) serves queries. The `AspenDnsClient` syncs records to local cache via iroh-docs P2P replication.

**Rationale**: `aspen-dns` is a fully built DNS system with:

- `DnsStore` trait + `AspenDnsStore` (Raft KV-backed CRUD for dns:* keys)
- `DnsProtocolServer` (hickory-server with zone-based authority, upstream forwarding)
- `AspenDnsClient` (iroh-docs sync → local cache → instant lookups)
- `AspenDnsAuthority` (hickory Authority impl backed by local cache)
- Full record type support (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA)
- Wildcard resolution, zone management, validation

Building a separate MagicDNS stub resolver would duplicate this. Instead, service mesh publishes create real DNS records in the `aspen` zone, and the existing DNS infrastructure serves them.

**Integration flow:**

```
aspen net publish mydb --port 5432
  → Write /_sys/net/svc/mydb (service registry)
  → DnsStore::set_record(SRV _tcp.mydb.aspen → endpoint:5432)
  → DnsStore::set_record(A mydb.aspen → 127.0.0.x)
  → Raft → DocsExporter → iroh-docs sync
  → All AspenDnsClients get the record via P2P
  → DnsProtocolServer answers: dig mydb.aspen @localhost:5353
```

**Alternative considered**: Custom MagicDNS stub in aspen-net — rejected because aspen-dns already exists with full DNS protocol support, zone management, and P2P sync.

### 8. Standalone daemon binary

**Decision**: `aspen net up` starts a long-running daemon process that orchestrates SOCKS5 proxy, DNS resolver, service auto-publishing, and peer map sync. It connects to the cluster as a client (using a cluster ticket) and doesn't need to be an aspen-node.

**Rationale**: Most users of the service mesh won't be running full aspen-nodes. They're developers or applications that want to reach services in the cluster. The daemon is a thin client that holds an iroh endpoint, watches the service registry, and runs local proxies. This is analogous to `tailscaled` being separate from the Tailscale coordination server.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        aspen-net daemon                          │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────┐    │
│  │ SOCKS5 proxy │  │ aspen-dns    │  │ Service publisher  │    │
│  │ :1080        │  │ :5353        │  │ (auto-register     │    │
│  │              │  │ DnsProtocol- │  │  local services)   │    │
│  │ Resolves     │  │ Server +     │  │                    │    │
│  │ names via    │  │ AspenDns-    │  │ Watches Raft KV    │    │
│  │ registry     │  │ Client       │  │ for peer changes   │    │
│  └──────┬───────┘  └──────────────┘  └────────────────────┘    │
│         │                                                       │
│  ┌──────┴──────────────────────────────────────────────────┐    │
│  │  Name Resolver                                           │    │
│  │  service name → (endpoint_id, port)                      │    │
│  │  Reads /_sys/net/svc/{name} via Client RPC               │    │
│  └──────┬──────────────────────────────────────────────────┘    │
│         │                                                       │
│  ┌──────┴──────────────────────────────────────────────────┐    │
│  │  Token Verifier (aspen-auth)                              │    │
│  │  verify(token) → Ok                                       │    │
│  │  authorize(token, NetConnect { svc }) → Ok|Unauthorized   │    │
│  │  Local, offline, pure crypto — no I/O                     │    │
│  └──────┬──────────────────────────────────────────────────┘    │
│         │                                                       │
│  ┌──────┴──────────────────────────────────────────────────┐    │
│  │  DownstreamProxy (from iroh-proxy-utils)                  │    │
│  │  create_tunnel() for SOCKS5 connections                   │    │
│  │  forward_tcp_listener() for port forwarding               │    │
│  │  Connection pooling, metrics, auth — all built-in         │    │
│  └──────┬──────────────────────────────────────────────────┘    │
│         │                                                       │
│  ┌──────┴──────────────────────────────────────────────────┐    │
│  │  Iroh Endpoint (QUIC, NAT traversal, relay)               │    │
│  └──────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

**Data flow — SOCKS5 connection:**

```
App → SOCKS5 handshake ("connect mydb.aspen:5432")
  → Resolve "mydb" from registry → endpoint_id + port
  → Token check: verifier.authorize(token, NetConnect { service: "mydb" })
    ← pure crypto, no I/O, no Raft read
  → DownstreamProxy::create_tunnel(endpoint_id, "localhost:5432")
  → QUIC CONNECT tunnel to remote UpstreamProxy
  → UpstreamProxy forwards to localhost:5432 on remote node
  → Bidirectional byte stream: App ↔ SOCKS5 ↔ QUIC ↔ UpstreamProxy ↔ postgres
```

**Data flow — port forwarding:**

```
$ aspen net forward 5432:mydb --token <token>
  → Resolve "mydb" → endpoint_id + port
  → Token check: verifier.authorize(token, NetConnect { service: "mydb" })
  → DownstreamProxy::new(endpoint, pool_opts)
  → mode = ProxyMode::Tcp(EndpointAuthority { endpoint_id, authority: "localhost:5432" })
  → proxy.forward_tcp_listener(TcpListener::bind(":5432"), mode)
  → Every TCP connection to localhost:5432 is tunneled automatically
```

## Risks / Trade-offs

- **[TCP only]** No UDP support in initial implementation. DNS queries, game servers, and VoIP won't work through the mesh. → Mitigation: Add QUIC datagram tunneling in a future phase. Most service-to-service communication is TCP.

- **[SOCKS5 app support varies]** Not all applications support SOCKS5 natively. → Mitigation: Port forwarding works universally. DNS resolution + transparent proxy covers most remaining cases. `proxychains` wraps arbitrary apps.

- **[DNS resolver needs privileges on port 53]** Standard DNS port requires root. → Mitigation: Default to port 5353 (mDNS port, often unprivileged). Users can configure port 53 with CAP_NET_BIND_SERVICE or run through systemd socket activation.

- **[Token distribution]** Capability tokens must be pre-created and distributed to daemon operators. → Mitigation: `aspen-cli token generate` already exists. Tokens can be generated with appropriate `NetConnect`/`NetPublish` scopes and shared out-of-band (same as cluster tickets). Delegation chains allow team leads to create scoped tokens without admin involvement.

- **[Token revocation latency]** Revoked tokens remain valid until the revocation propagates. → Mitigation: Revocation store is Raft-backed (`KeyValueRevocationStore` already exists). Daemon can periodically refresh its revocation list. For most use cases, token expiration (lifetime) is sufficient.

- **[iroh-proxy-utils dependency]** We depend on the `iroh-http-proxy/1` protocol remaining stable. → Mitigation: Already vendored. Version is pinned. Protocol is simple (HTTP/1.1 CONNECT).

## Crate Structure

```
NEW:
  crates/aspen-net/
  ├── src/
  │   ├── lib.rs              ← Public API, feature re-exports
  │   ├── registry.rs         ← Service CRUD (KV wrappers + auto DNS record creation)
  │   ├── resolver.rs         ← Name → (endpoint_id, port) resolution
  │   ├── socks5.rs           ← SOCKS5 server (RFC 1928 handshake + tunnel)
  │   ├── forward.rs          ← Port forwarding (thin wrapper over DownstreamProxy)
  │   ├── daemon.rs           ← Orchestration (SOCKS5 + AspenDnsClient + registry watcher)
  │   ├── auth.rs             ← Token verification wrapper (delegates to aspen-auth)
  │   ├── types.rs            ← ServiceEntry, NodeEntry
  │   ├── constants.rs        ← Tiger Style bounds
  │   └── verified/
  │       ├── mod.rs
  │       └── service_name.rs ← Pure service name validation
  ├── bin/
  │   └── aspen-net.rs        ← Standalone daemon binary
  └── Cargo.toml

EXTERNAL (sibling repo, already built):
  ../aspen-dns/crates/aspen-dns/     ← DnsStore, DnsProtocolServer, AspenDnsClient
  ../aspen-dns/crates/aspen-dns-plugin/  ← WASM plugin (DnsSetRecord, DnsGetRecord)

MODIFIED:
  crates/aspen-auth/           ← New Capability variants (NetConnect, NetPublish, NetAdmin)
                                 New Operation variants (NetConnect, NetPublish, NetUnpublish, NetAdmin)
                                 Containment rules for delegation
  crates/aspen-client-api/     ← Net RPC variants
  crates/aspen-cli/            ← `aspen net` subcommands
  crates/aspen-rpc-handlers/   ← Net handler registration
  crates/aspen-constants/      ← Net constants
  crates/aspen/                ← Node: register UpstreamProxy on router when `net` feature enabled
```
