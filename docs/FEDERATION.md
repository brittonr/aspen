# Aspen Federation Guide

> How independent Aspen clusters discover each other, share content, and
> synchronize resources across organizational boundaries — without HTTP,
> DNS, or any central authority.

## Table of Contents

1. [Overview](#overview)
2. [Core Concepts](#core-concepts)
3. [Architecture](#architecture)
4. [Cluster Identity](#cluster-identity)
5. [Trust Management](#trust-management)
6. [Application Registry](#application-registry)
7. [Discovery](#discovery)
8. [Gossip](#gossip)
9. [Sync Protocol](#sync-protocol)
10. [Capability-Aware Dispatch](#capability-aware-dispatch)
11. [Cross-Cluster Proxying](#cross-cluster-proxying)
12. [Federated IDs](#federated-ids)
13. [Configuration](#configuration)
14. [Resource Limits](#resource-limits)
15. [Example: Forge Federation](#example-forge-federation)
16. [Developing Federated Applications](#developing-federated-applications)
17. [Feature Flags](#feature-flags)
18. [Crate Map](#crate-map)

---

## Overview

Aspen federation connects independent clusters into a cooperative network.
Each cluster remains **fully sovereign** — it owns its data, runs its own Raft
consensus, and operates normally when offline. Federation is additive: connecting
to other clusters adds capabilities but is never required.

Key properties:

- **Pure P2P**: Built on iroh QUIC transport. No HTTP, no DNS, no central relay.
- **Self-sovereign identity**: Ed25519 keypairs. No external identity provider.
- **Strong consistency within cluster**: Raft for authoritative state.
- **Eventual consistency across clusters**: Pull-based sync with cryptographic verification.
- **DHT-based discovery**: BitTorrent Mainline DHT (BEP-44) for finding peers.

Federation follows the **FoundationDB "unbundled database"** philosophy: the core
provides minimal, robust primitives (KV, blobs, consensus, transport), and higher-level
features like Forge or CI are stateless layers that store all state through those
primitives. Layers are cluster-local; federation happens at the application level
using shared primitives.

---

## Core Concepts

### Cluster-Level Layers, Not Global Layers

Each cluster has its own independent namespace. The same path (e.g., `apps/forge`)
may map to different binary prefixes on different clusters. This is intentional.

```
Cluster A                              Cluster B
+--------------------------+          +--------------------------+
| Directory Layer          |          | Directory Layer          |
| apps/forge -> 0x15       |          | apps/forge -> 0x23       |
|                          |          |                          |
| Forge instances          |          | Forge instances          |
| with local state         |          | with local state         |
+--------------------------+          +--------------------------+
         ^                                    ^
         +-------- App-level sync ------------+
                 (Forge <-> Forge)
```

Why not global layers?

1. **CAP theorem**: Can't have strong consistency across partitioned clusters.
2. **Sovereignty**: Clusters shouldn't depend on each other for basic operations.
3. **Offline-first**: A cluster must work when peers are unreachable.
4. **Content-addressed**: Iroh is content-addressed, not location-addressed.

### Pull-Based Sync

Cross-cluster synchronization is always pull-based:

1. A cluster **discovers** a peer via DHT or gossip.
2. It **queries** the peer's resource state (what refs/objects exist).
3. It **fetches** missing objects on demand.
4. All fetched data is **verified** (cluster signatures, delegate signatures, content hashes).

No cluster can push data to another without the receiver explicitly requesting it.

### Three Verification Layers

All cross-cluster data passes through:

1. **Cluster signature**: Every announcement is signed by the originating cluster's Ed25519 key.
2. **Delegate signature**: Canonical refs (e.g., Forge branch heads) are signed by authorized delegates.
3. **Content hash**: All objects are BLAKE3-hashed. Tampering is detected immediately.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Aspen Node                                │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                   Handler Registry                          │ │
│  │  Dispatches requests → tries local handlers first           │ │
│  │  → tries cross-cluster proxy if no local handler            │ │
│  │  → returns CapabilityUnavailable with hints                 │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│  ┌───────────────────────────┼───────────────────────────────┐  │
│  │               Federation Layer                             │  │
│  │                                                            │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  │  │
│  │  │ Cluster      │  │ App          │  │ Trust           │  │  │
│  │  │ Identity     │  │ Registry     │  │ Manager         │  │  │
│  │  │ (Ed25519)    │  │ (manifests)  │  │ (per-cluster)   │  │  │
│  │  └──────────────┘  └──────────────┘  └────────────────┘  │  │
│  │                                                            │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  │  │
│  │  │ DHT          │  │ Gossip       │  │ Sync           │  │  │
│  │  │ Discovery    │  │ Service      │  │ Protocol       │  │  │
│  │  │ (BEP-44)     │  │ (iroh-gossip)│  │ (QUIC/ALPN)    │  │  │
│  │  └──────────────┘  └──────────────┘  └────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────┼───────────────────────────────┐  │
│  │               Core Primitives                              │  │
│  │  KV Store (Raft)  │  Blob Store (iroh-blobs)  │  Gossip   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    Iroh Transport (QUIC)                    │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Cluster Identity

Each cluster has a **stable Ed25519 keypair** that represents the organization.
Unlike node keys (which are per-node and ephemeral), the cluster key is
long-lived and shared among all nodes in a cluster.

**Source**: `crates/aspen-federation/src/identity.rs`

### Creating an Identity

```rust
use aspen_federation::ClusterIdentity;

// Generate a fresh identity
let identity = ClusterIdentity::generate("my-organization".to_string())
    .with_description("Production cluster in EU-West".to_string());

// Save the secret key for persistence
let key_hex = identity.secret_key_hex();
// Store key_hex securely (config file, secrets manager)

// Restore from saved key
let restored = ClusterIdentity::from_hex_key(&key_hex, "my-organization".to_string())
    .expect("valid hex key");
assert_eq!(identity.public_key(), restored.public_key());
```

### Signed Identity Documents

To share your identity with other clusters, create a signed document:

```rust
let signed = identity.to_signed();

// The signed document contains:
// - public_key, name, description, created_at_ms
// - signature (Ed25519 over the serialized document)

// Recipients verify it:
assert!(signed.verify());
println!("Cluster: {} ({})", signed.name(), signed.public_key());
```

### Key Management

| Property | Guidance |
|----------|----------|
| Generation | Once, when the cluster is first created |
| Storage | Config file or secrets manager (e.g., Vault) |
| Distribution | Shared among all nodes in the cluster via config |
| Rotation | Avoid — requires careful migration of all trust relationships |
| Backup | Essential — losing the key means losing the cluster's federated identity |

---

## Trust Management

Trust relationships control which clusters can access your federated resources.

**Source**: `crates/aspen-federation/src/trust.rs`

### Trust Levels

| Level | Access | Use Case |
|-------|--------|----------|
| **Trusted** | Full access to all federated resources | Known organizations, business partners |
| **Public** | Public resources only | Anyone on the network |
| **Blocked** | No access | Spammers, hostile clusters |

### Using the Trust Manager

```rust
use aspen_federation::{TrustManager, TrustLevel};

let manager = TrustManager::new();

// Trust a cluster
manager.add_trusted(peer_key, "acme-corp".to_string(), Some("Business partner".to_string()));

// Block a cluster
manager.block(spammer_key);

// Check access
assert_eq!(manager.trust_level(&peer_key), TrustLevel::Trusted);
assert_eq!(manager.trust_level(&unknown_key), TrustLevel::Public);
assert_eq!(manager.trust_level(&spammer_key), TrustLevel::Blocked);

// Resource access checks combine trust level with resource mode
use aspen_federation::types::FederationMode;
assert!(manager.can_access_resource(&peer_key, &FederationMode::AllowList));
assert!(manager.can_access_resource(&unknown_key, &FederationMode::Public));
assert!(!manager.can_access_resource(&unknown_key, &FederationMode::AllowList));
assert!(!manager.can_access_resource(&spammer_key, &FederationMode::Public));
```

### Trust Requests

Clusters can request trust from each other:

```rust
// Remote cluster sends a signed identity
let request_identity = remote_cluster.to_signed();

// Add to pending queue (signature verified automatically)
manager.add_trust_request(request_identity, Some("Please federate with us".to_string()));

// Review pending requests
for req in manager.list_pending_requests() {
    println!("Request from: {} - {}", req.identity.name(), req.message.unwrap_or_default());
}

// Accept or reject
manager.accept_trust_request(&requester_key);  // Moves to trusted
manager.reject_trust_request(&other_key);       // Removed from queue
```

Trust requests expire after 1 hour (`TRUST_REQUEST_EXPIRY_SECS`).

---

## Application Registry

The `AppRegistry` tracks which applications are installed on a cluster.
This information is shared during discovery and gossip so that other clusters
know what capabilities are available.

**Source**: `crates/aspen-core/src/app_registry.rs`

### Registering Applications

```rust
use aspen_core::app_registry::{AppManifest, AppRegistry};

let registry = AppRegistry::new();

// Register Forge
registry.register(
    AppManifest::new("forge", "1.0.0")
        .with_name("Aspen Forge")
        .with_capabilities(vec!["git", "issues", "patches"])
);

// Register CI
registry.register(
    AppManifest::new("ci", "2.0.0")
        .with_name("Aspen CI")
        .with_capabilities(vec!["build", "test", "deploy"])
);

// Query
assert!(registry.has_app("forge"));
let git_apps = registry.find_apps_with_capability("git");
assert_eq!(git_apps[0].app_id, "forge");
```

### How Handlers Register

Request handlers declare their `app_id()` during construction. The handler registry
auto-populates the `AppRegistry` from loaded handlers, so applications are
advertised as soon as their handlers are loaded.

```rust
impl RequestHandler for ForgeRepoHandler {
    fn app_id(&self) -> Option<&'static str> {
        Some("forge")
    }
    // ...
}
```

### Announcement Integration

The registry converts to/from announcement lists for gossip:

```rust
// Outbound: include in ClusterAnnouncement
let apps = registry.to_announcement_list();

// Inbound: reconstruct from received announcement
let remote_registry = AppRegistry::from_announcement_list(received_apps);
```

---

## Discovery

The discovery service uses the **BitTorrent Mainline DHT** (BEP-44 mutable items)
to find clusters without any central authority.

**Source**: `crates/aspen-federation/src/discovery.rs`

### How It Works

1. Each cluster publishes a **ClusterAnnouncement** to the DHT, keyed by
   `sha256("aspen:cluster:v1:" || cluster_pubkey)[..20]`.

2. Clusters looking for peers query the DHT by cluster public key or by
   resource federated ID.

3. Discovered clusters are cached locally (LRU, up to 1024 entries).

### ClusterAnnouncement

The announcement contains everything needed to connect:

```rust
pub struct ClusterAnnouncement {
    pub cluster_key: PublicKey,       // Cluster identity
    pub cluster_name: String,         // Human-readable name
    pub node_keys: Vec<PublicKey>,     // Iroh node keys (for connection)
    pub relay_urls: Vec<String>,      // Relay URLs for NAT traversal
    pub apps: Vec<AppManifest>,       // Installed applications
    pub timestamp_ms: u64,            // HLC timestamp
    pub signature: Signature,         // Ed25519 signature over the above
}
```

### Resource Announcements

Individual resources (e.g., a specific Forge repository) can be announced
separately for fine-grained discovery:

```rust
pub struct ResourceAnnouncement {
    pub fed_id: FederatedId,          // Global resource ID
    pub resource_type: String,        // e.g., "forge:repo"
    pub cluster_key: PublicKey,       // Origin cluster
    pub seeders: Vec<PublicKey>,      // Clusters that have this resource
    pub timestamp_ms: u64,
    pub signature: Signature,
}
```

### DHT Key Derivation

Keys are deterministic 20-byte info-hashes:

| Resource | Key Formula |
|----------|-------------|
| Cluster | `sha256("aspen:cluster:v1:" \|\| cluster_pubkey)[..20]` |
| Federated resource | `sha256("aspen:fed:v1:" \|\| origin_key \|\| local_id)[..20]` |

### Feature Flag

Actual DHT operations require the `global-discovery` feature flag. Without it,
discovery calls are logged but don't perform real DHT queries — useful for
testing and local-only deployments.

---

## Gossip

Real-time federation events are propagated via **iroh-gossip** topics.

**Source**: `crates/aspen-federation/src/gossip/`

### Architecture

```
gossip/
├── mod.rs           — FederationGossipService (orchestrator)
├── messages.rs      — SignedFederationMessage, FederationGossipMessage
├── announcer.rs     — Periodic announcement broadcaster
├── receiver.rs      — Incoming message processor
├── rate_limiter.rs  — Token bucket rate limiting
└── events.rs        — FederationEvent enum
```

### Message Types

All gossip messages are signed by the sending cluster:

```rust
pub enum FederationGossipMessage {
    /// Cluster came online or updated its capabilities
    ClusterAnnounce(ClusterAnnouncement),
    /// New federated resource available
    ResourceAvailable(ResourceAnnouncement),
    /// Resource updated (new refs, etc.)
    ResourceUpdated { fed_id: FederatedId, timestamp_ms: u64 },
    /// Resource removed from federation
    ResourceRemoved { fed_id: FederatedId, timestamp_ms: u64 },
}
```

### Rate Limiting

Gossip is rate-limited to prevent flooding:

| Limit | Value | Scope |
|-------|-------|-------|
| Per-cluster rate | 12 messages/minute | Per sending cluster |
| Per-cluster burst | 5 messages | Instantaneous burst allowance |
| Global rate | 600 messages/minute | Total across all clusters |
| Global burst | 100 messages | Global instantaneous burst |
| Tracked clusters | 512 | LRU eviction for rate limit state |

### Events

The gossip receiver emits `FederationEvent`s that applications can subscribe to:

```rust
pub enum FederationEvent {
    ClusterDiscovered { key: PublicKey, announcement: ClusterAnnouncement },
    ClusterUpdated { key: PublicKey, announcement: ClusterAnnouncement },
    ResourceAvailable { fed_id: FederatedId, announcement: ResourceAnnouncement },
    ResourceUpdated { fed_id: FederatedId, timestamp_ms: u64 },
    ResourceRemoved { fed_id: FederatedId },
    MessageRejected { reason: String },
}
```

---

## Sync Protocol

The federation sync protocol handles actual data transfer between clusters
over QUIC streams with ALPN routing.

**Source**: `crates/aspen-federation/src/sync/`

### ALPN

```
/aspen/federation/1
```

### Protocol Flow

```
Initiator                              Responder
    │                                      │
    │──── Handshake (identity, caps) ─────►│
    │◄─── Handshake (identity, caps, trust)│
    │                                      │
    │──── ListResources (type, cursor) ───►│
    │◄─── ResourceList (resources, next) ──│
    │                                      │
    │──── GetResourceState (fed_id) ──────►│
    │◄─── ResourceState (heads, metadata) ─│
    │                                      │
    │──── SyncObjects (fed_id, want, have)►│
    │◄─── Objects (data, has_more) ────────│
    │                                      │
    │──── VerifyRefUpdate (fed_id, sig) ──►│
    │◄─── VerifyResult (valid, error) ─────│
```

### Request Types

```rust
pub enum FederationRequest {
    /// Exchange identities and negotiate capabilities
    Handshake { identity: SignedClusterIdentity, protocol_version: u8, capabilities: Vec<String> },
    /// List federated resources (with optional type filter and pagination)
    ListResources { resource_type: Option<String>, cursor: Option<String>, limit: u32 },
    /// Get current state of a specific resource (heads, metadata)
    GetResourceState { fed_id: FederatedId },
    /// Request missing objects for a resource
    SyncObjects { fed_id: FederatedId, want_types: Vec<String>, have_hashes: Vec<[u8; 32]>, limit: u32 },
    /// Verify a ref update signature from a delegate
    VerifyRefUpdate { fed_id: FederatedId, ref_name: String, new_hash: [u8; 32], signature: Signature, signer: [u8; 32] },
}
```

### Wire Format

Messages use **postcard serialization** with **length-prefixed framing**:

```
┌──────────────┬──────────────────────┐
│ length: u32  │ postcard-encoded msg │
└──────────────┴──────────────────────┘
```

### Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_MESSAGE_SIZE` | 16 MB | Single message size limit |
| `MAX_OBJECTS_PER_SYNC` | 1,000 | Objects per sync request |
| `MAX_RESOURCES_PER_LIST` | 1,000 | Resources per list response |
| `MAX_FEDERATION_CONNECTIONS` | 64 | Concurrent federation connections |
| `MAX_STREAMS_PER_CONNECTION` | 16 | Streams per connection |
| `HANDSHAKE_TIMEOUT` | 10s | Handshake completion deadline |
| `REQUEST_TIMEOUT` | 60s | Per-request deadline |
| `MESSAGE_PROCESSING_TIMEOUT` | 5s | Per-message processing deadline |

---

## Capability-Aware Dispatch

When a request arrives that requires an application not loaded on the local
cluster, the handler registry responds gracefully.

**Source**: `crates/aspen-rpc-handlers/src/registry.rs`

### How It Works

1. Every `ClientRpcRequest` declares its `required_app()` — e.g., `ForgeCreateRepo` → `Some("forge")`.
2. The handler registry checks if any loaded handler can serve the request.
3. If no handler matches and the request has a `required_app`:
   - Try **cross-cluster proxying** (if enabled).
   - If proxying fails or is disabled, return `CapabilityUnavailable` with hints.

```rust
// In handler registry dispatch:
if let Some(app_id) = request.required_app() {
    if !local_handlers_can_serve {
        // Try proxy first
        if proxy_enabled {
            if let Ok(Some(response)) = proxy_service.proxy_request(...).await {
                return Ok(response);
            }
        }
        // Return capability unavailable with hints
        return Ok(ClientRpcResponse::CapabilityUnavailable(
            CapabilityUnavailableResponse {
                required_app: app_id.to_string(),
                message: format!("App '{}' not loaded on this cluster", app_id),
                hints: vec![],  // Could include known clusters with this app
            }
        ));
    }
}
```

### Request Classification

Core requests (KV, blob, cluster operations) have `required_app() → None` and
are always handled locally. Plugin/application requests return their app ID:

| Request Family | `required_app()` |
|---------------|-------------------|
| KV, Blob, Cluster, Coordination | `None` (always available) |
| Forge operations | `Some("forge")` |
| CI operations | `Some("ci")` |
| DNS operations | `Some("dns")` |
| Automerge operations | `Some("automerge")` |
| Secrets operations | `Some("secrets")` |

---

## Cross-Cluster Proxying

When a request can't be served locally, the `ProxyService` can forward it
to a capable cluster.

**Source**: `crates/aspen-rpc-handlers/src/proxy.rs`

### Proxy Flow

1. Handler registry determines no local handler exists for the request.
2. `ProxyService::proxy_request()` is called with the request, app ID, and current hop count.
3. Proxy looks up known clusters with the required app (from gossip/discovery).
4. Request is forwarded with `proxy_hops + 1` to prevent loops.
5. Response is returned transparently to the original caller.

### Loop Prevention

The `AuthenticatedRequest` carries a `proxy_hops: u8` field. Each proxy
increments this counter. Requests exceeding `MAX_PROXY_HOPS` are rejected.

```rust
pub struct AuthenticatedRequest {
    pub request: ClientRpcRequest,
    pub token: Option<CapabilityToken>,
    pub proxy_hops: u8,  // Incremented at each proxy hop
}
```

---

## Federated IDs

Resources gain global uniqueness through origin-prefixed identifiers.

**Source**: `crates/aspen-federation/src/types.rs`

### Format

```
{origin_cluster_public_key}:{local_id_hex}
```

A `FederatedId` is 64 bytes: 32 bytes for the origin cluster's public key +
32 bytes for the local identifier.

### Authority

The origin key provides an authority anchor:

- The origin cluster controls **canonical refs** (via delegate signatures).
- Other clusters can **mirror** content but cannot modify authority.
- Verification chains back to the origin cluster's identity.

### Usage

```rust
use aspen_federation::FederatedId;

// Create from components
let fed_id = FederatedId::new(origin_key, local_id_bytes);

// Create from a BLAKE3 hash (common for Forge repos)
let fed_id = FederatedId::from_blake3(origin_key, blake3::hash(b"repo-identity"));

// DHT lookup key
let infohash = fed_id.to_dht_infohash();  // 20-byte SHA256 truncation

// Gossip topic
let topic = fed_id.to_gossip_topic();  // BLAKE3-based TopicId

// String roundtrip
let s = fed_id.to_string();
let parsed: FederatedId = s.parse().unwrap();
assert_eq!(parsed, fed_id);
```

### Per-Resource Federation Settings

Each resource can independently control its federation mode:

```rust
use aspen_federation::types::{FederationSettings, FederationMode};

// Public — anyone can discover and sync
let public = FederationSettings::public();

// Allowlist — only specific clusters
let restricted = FederationSettings::allowlist(vec![partner_key_1, partner_key_2]);

// Disabled — local only
let private = FederationSettings::disabled();

// Check access
assert!(public.is_cluster_allowed(&any_key));
assert!(restricted.is_cluster_allowed(&partner_key_1));
assert!(!restricted.is_cluster_allowed(&random_key));
```

---

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ASPEN_CLUSTER_KEY` | Hex-encoded Ed25519 secret key (64 chars) | Generated if absent |
| `ASPEN_CLUSTER_NAME` | Human-readable cluster name | `"aspen"` |
| `ASPEN_PROXY_ENABLED` | Enable cross-cluster request proxying | `false` |
| `ASPEN_PROXY_MAX_CONNECTIONS` | Max concurrent proxy connections | `128` |

### Node Configuration (TOML)

```toml
[cluster]
cookie = "my-cluster-secret"

[federation]
# Cluster identity (generate once, share across all nodes)
cluster_key = "abcdef0123456789..."  # 64 hex chars
cluster_name = "my-organization"

# Trust settings
trusted_clusters = [
    "deadbeef01234567...",  # Partner A's public key
    "cafebabe01234567...",  # Partner B's public key
]

# Proxy settings
[federation.proxy]
enabled = true
max_proxy_hops = 2
```

---

## Resource Limits

All federation operations have fixed bounds (Tiger Style) to prevent resource
exhaustion from malicious or misconfigured peers.

| Resource | Limit | Location |
|----------|-------|----------|
| Apps per cluster | 32 | `app_registry::MAX_APPS_PER_CLUSTER` |
| Capabilities per app | 16 | `app_registry::MAX_CAPABILITIES_PER_APP` |
| App ID length | 64 chars | `app_registry::MAX_APP_ID_LENGTH` |
| App name length | 128 chars | `app_registry::MAX_APP_NAME_LENGTH` |
| Capability string length | 64 chars | `app_registry::MAX_CAPABILITY_LENGTH` |
| Trusted clusters | 256 | `trust::MAX_TRUSTED_CLUSTERS` |
| Blocked clusters | 256 | `trust::MAX_BLOCKED_CLUSTERS` |
| Pending trust requests | 64 | `trust::MAX_PENDING_REQUESTS` |
| Trust request expiry | 1 hour | `trust::TRUST_REQUEST_EXPIRY_SECS` |
| Discovered clusters cache | 1,024 | `discovery::MAX_TRACKED_CLUSTERS` |
| Discovered seeders cache | 10,000 | `discovery::MAX_TRACKED_SEEDERS` |
| Gossip rate per cluster | 12/min (burst: 5) | `gossip::rate_limiter` |
| Gossip rate global | 600/min (burst: 100) | `gossip::rate_limiter` |
| Gossip tracked clusters | 512 | `gossip::rate_limiter` |
| Cluster name | 128 chars | `identity::MAX_CLUSTER_NAME_LEN` |
| Cluster description | 1,024 chars | `identity::MAX_CLUSTER_DESCRIPTION_LEN` |
| Allowed clusters per resource | 256 | `types::MAX_ALLOWED_CLUSTERS` |

All limits use **silent truncation** on input — exceeding a limit never causes
an error; the excess is simply dropped. This ensures robust handling of
potentially malicious data from untrusted peers.

---

## Example: Forge Federation

Forge (Aspen's decentralized Git hosting) demonstrates the full federation
pattern:

### Setup

```
Cluster A (alice)                    Cluster B (bob)
+----------------------------+      +----------------------------+
| Apps: [forge, ci]          |      | Apps: [forge]              |
|                            |      |                            |
| Forge repos:               |      | Forge repos:               |
|   aspen (local)            |      |   aspen (mirror)           |
|     objects (blobs)        |      |     objects (blobs)        |
|     refs (KV)              |      |     refs (KV)              |
|   remotes/bob/aspen -------+------+--                          |
+----------------------------+      +----------------------------+
```

### Federation Flow

1. **Alice creates a repo** on her cluster. State stored in local KV/blobs.

2. **Alice federates the repo**: Sets `FederationSettings::public()` (or allowlist).
   The repo gets a `FederatedId` = `alice_cluster_key:blake3(repo_identity)`.

3. **Announcement**: Alice's gossip service broadcasts a `ResourceAvailable` message
   to the federation gossip topic. The DHT is also updated with the resource
   announcement.

4. **Bob discovers the repo**: Bob's discovery service finds Alice's cluster
   (either via DHT lookup or gossip). Bob verifies Alice's cluster signature.

5. **Bob adds as remote**: `git remote add alice aspen://alice-cluster/aspen`

6. **Ref sync**: Bob's Forge queries Alice for resource state (`GetResourceState`),
   gets the current ref heads (e.g., `refs/heads/main → abc123`).

7. **Object sync**: Bob requests missing objects (`SyncObjects` with `have_hashes`).
   Objects are content-addressed BLAKE3 blobs — iroh-blobs handles dedup and transfer.

8. **Verification**: Bob verifies delegate signatures on canonical refs and
   BLAKE3 hashes on all objects.

### Key Insight

The layers are independent (different directory prefixes on each cluster).
The **content** is the same (content-addressed by hash). Federation happens at
the **Forge application level**, not the infrastructure level.

---

## Developing Federated Applications

Applications that want to participate in federation should follow these patterns:

### 1. Declare an App ID

Every handler should return an `app_id()` for capability-aware dispatch:

```rust
impl RequestHandler for MyAppHandler {
    fn app_id(&self) -> Option<&'static str> {
        Some("my-app")
    }
}
```

### 2. Register an AppManifest

On startup, register your application's manifest:

```rust
let manifest = AppManifest::new("my-app", "1.0.0")
    .with_name("My Application")
    .with_capabilities(vec!["feature-a", "feature-b"]);
ctx.app_registry.register(manifest);
```

### 3. Store All State in Core Primitives

Like FoundationDB layers, your application should be **stateless** — all
persistent state goes through KV and blob stores:

```rust
// Good: state in KV
ctx.kv.write(b"my-app:config:setting", value).await?;

// Bad: state in memory (lost on restart)
self.internal_cache.insert(key, value);  // Don't do this!
```

### 4. Use Federated IDs for Global Resources

Resources that can be shared across clusters should use `FederatedId`:

```rust
let fed_id = FederatedId::new(
    ctx.cluster_identity.public_key(),
    blake3::hash(resource_identity.as_bytes()).into(),
);
```

### 5. Respect Federation Settings

Check resource federation settings before serving cross-cluster requests:

```rust
let settings = get_resource_settings(&fed_id).await?;
if !settings.is_cluster_allowed(&requester_key) {
    return Err(FederationError::AccessDenied);
}
```

### 6. Sign Critical Data

Sign authoritative data with cluster or delegate keys:

```rust
let signature = ctx.cluster_identity.sign(&canonical_data);
```

---

## Feature Flags

| Feature | Crate | Effect |
|---------|-------|--------|
| `global-discovery` | `aspen-federation` | Enables actual DHT operations via `mainline` crate. Without this, discovery is logged but no-op. |
| `forge` | Root crate | Includes Forge handlers and federation integration |
| `proxy` | Root crate | Enables cross-cluster HTTP proxying via `aspen-proxy` |

For testing, omit `global-discovery` to avoid real DHT traffic.

---

## Crate Map

| Crate | Purpose |
|-------|---------|
| `aspen-federation` | Core federation types: identity, trust, discovery, gossip, sync protocol, resolver |
| `aspen-core` (app_registry) | `AppManifest` and `AppRegistry` (shared to avoid circular deps) |
| `aspen-client-api` | `CapabilityUnavailable` response, `required_app()`, `proxy_hops` |
| `aspen-rpc-handlers` | Handler registry dispatch with capability-aware routing and `ProxyService` |
| `aspen-rpc-core` | `RequestHandler` trait with `app_id()` method |
| `aspen-cluster` | Federation module re-exports, cluster config for federation settings |
| `aspen-proxy` | HTTP proxying over iroh QUIC (for the `proxy` CLI subcommand) |
| `aspen-dht-discovery` | DHT integration used by `aspen-federation/discovery.rs` |

### Module Structure within `aspen-federation`

```
crates/aspen-federation/src/
├── lib.rs             — Public API re-exports
├── identity.rs        — ClusterIdentity, SignedClusterIdentity
├── types.rs           — FederatedId, FederationMode, FederationSettings
├── app_registry.rs    — Re-exports from aspen-core
├── trust.rs           — TrustManager, TrustLevel, TrustRequest
├── discovery.rs       — FederationDiscoveryService, ClusterAnnouncement
├── resolver.rs        — FederationResourceResolver (direct + sharded)
├── gossip/
│   ├── mod.rs         — FederationGossipService
│   ├── messages.rs    — SignedFederationMessage, FederationGossipMessage
│   ├── announcer.rs   — Periodic announcement broadcaster
│   ├── receiver.rs    — Incoming message processor
│   ├── rate_limiter.rs — Token bucket rate limiting
│   └── events.rs      — FederationEvent enum
└── sync/
    ├── mod.rs         — Constants, ALPN definition
    ├── types.rs       — FederationRequest, FederationResponse
    ├── wire.rs        — Length-prefixed postcard framing
    ├── handler.rs     — FederationProtocolHandler (server side)
    └── client.rs      — connect_to_cluster, sync functions (client side)
```
