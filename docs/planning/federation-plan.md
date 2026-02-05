# Federation Architecture Plan

This document outlines the architectural approach for Aspen's federation model and layer architecture.

## Decision: Cluster-Level Layers + Federation Primitives

Aspen adopts **cluster-level layers** with **federation primitives** at the application level, rather than federation-level layers that span clusters.

### Core Principle

Each cluster is sovereign. Layers (Directory, Subspace, Tuple, Index) operate within a single cluster. Federation is achieved through application-level protocols using shared primitives.

```
+-------------------------------------------------------------+
|                     Application Layer                        |
|   Forge <--------------------------------------> Forge       |
|   (app-level federation using primitives below)              |
+-------------------------------------------------------------+
                            |
                            v
+-------------------------------------------------------------+
|                 Federation Primitives                        |
|  +--------------+  +--------------+  +-------------------+   |
|  | Peer         |  | App          |  | Cross-Cluster     |   |
|  | Discovery    |  | Registry     |  | Sync (CRDTs)      |   |
|  +--------------+  +--------------+  +-------------------+   |
+-------------------------------------------------------------+
                            |
                            v
+-------------------------------------------------------------+
|               Cluster-Local Layers                           |
|  +--------------+  +--------------+  +-------------------+   |
|  | Directory    |  | Subspace     |  | Tuple             |   |
|  | (local)      |  | (local)      |  | (local)           |   |
|  +--------------+  +--------------+  +-------------------+   |
+-------------------------------------------------------------+
                            |
                            v
+-------------------------------------------------------------+
|               Core (Raft + Iroh + Storage)                   |
+-------------------------------------------------------------+
```

## Rationale

### Why NOT Federation-Level Layers

Federation-level layers would mean layers span clusters with global consensus:

```
# REJECTED: Global Directory Layer
+-------------------------------------------------------------+
|                 Global Directory Layer                       |
|         (consensus across all federated clusters)            |
|  +--------------------------------------------------------+ |
|  | apps/forge -> 0x15 (same everywhere)                   | |
|  | apps/ci    -> 0x16 (same everywhere)                   | |
|  +--------------------------------------------------------+ |
+-------------------------------------------------------------+
        ^               ^               ^
   Cluster A       Cluster B       Cluster C
```

Problems with this approach:

1. **CAP theorem violation**: Cannot have strong consistency across partitioned clusters
2. **Centralization risk**: Who owns the "global" state?
3. **Offline failure**: Cluster A cannot create directories if Cluster B is unreachable
4. **Conflicts with Iroh model**: Iroh is content-addressed, not location-addressed
5. **Sovereignty loss**: Clusters depend on each other for basic operations

### Why Cluster-Level Layers + Federation Primitives

```
Cluster A                              Cluster B
+--------------------------+          +--------------------------+
| Directory Layer          |          | Directory Layer          |
| +----------------------+ |          | +----------------------+ |
| | apps/forge -> 0x15   | |          | | apps/forge -> 0x23   | |
| | apps/ci    -> 0x16   | |          | | apps/ci    -> 0x24   | |
| +----------------------+ |          | +----------------------+ |
|                          |          |                          |
| Forge instances          |          | Forge instances          |
| with local state         |          | with local state         |
+--------------------------+          +--------------------------+
            ^                                    ^
            +-------- App-level sync ------------+
                    (Forge <-> Forge)
```

Benefits:

1. **Sovereignty**: Each cluster owns its layer state completely
2. **Offline-first**: Clusters work independently; federation is additive
3. **Strong local consistency**: Raft provides linearizability within cluster
4. **Eventual federation**: Cross-cluster sync via CRDTs and content-addressing
5. **Matches Iroh**: Content-addressed model, no global namespace
6. **Simpler layers**: Complexity is in apps, not infrastructure

## Comparison Table

| Aspect | Cluster-Level | Federation-Level |
| ------ | ------------- | ---------------- |
| Ownership | Each cluster owns its state | Shared/unclear |
| Offline operation | Full functionality | Degrades or fails |
| Consistency | Strong within, eventual across | Need cross-cluster consensus |
| Namespace | Same path = different prefix per cluster | Global uniform namespace |
| Complexity | Simple layers, complex apps | Complex layers, simpler apps |
| Decentralization | Preserved | Tends toward centralization |

## Layer Architecture

### Layers (Cluster-Local)

All layers operate within a single cluster's Raft consensus:

| Layer | Scope | Purpose |
| ----- | ----- | ------- |
| Tuple | Cluster | Order-preserving key encoding |
| Subspace | Cluster | Namespace isolation via prefixes |
| Directory | Cluster | Hierarchical namespace allocation |
| Index | Cluster | Secondary indexes with transactional guarantees |

### Layer API Location

```
crates/aspen-layer/
  src/
    lib.rs
    tuple.rs      # FoundationDB-compatible encoding
    subspace.rs   # Namespace isolation
    directory.rs  # Hierarchical allocation
    index.rs      # Secondary indexes
```

This crate provides the public API for application developers building on Aspen.

## Federation Primitives

Instead of federation-level layers, Aspen provides primitives that applications use to federate:

### 1. Peer Discovery

Find clusters and applications across the network:

```rust
// Find clusters running a specific application
let forge_peers = discovery.find_peers_with_app("forge").await?;

// Connect to a specific cluster by ticket
let peer = discovery.connect(ticket).await?;
```

### 2. App Registry

Each cluster maintains a registry of installed applications:

```rust
// Register an application on this cluster
cluster.registry().register("forge", AppManifest {
    version: "1.0.0",
    capabilities: vec!["git", "issues", "patches"],
    public_key: app_public_key,
}).await?;

// Query a remote cluster's applications
let remote_apps = peer.registry().list_apps().await?;
// Returns: ["forge", "ci", "custom-app"]

// Check if remote has specific app
if peer.registry().has_app("forge").await? {
    // Can federate with this cluster's Forge
}
```

### 3. Cross-Cluster Sync (CRDTs)

For application data that needs to sync across clusters:

```rust
// Sync a CRDT document with a remote peer
let sync = CrdtSync::new(local_store, remote_peer);
sync.sync_document("repos/aspen/metadata").await?;

// Or use iroh-docs for automatic CRDT replication
let doc = iroh_docs.create_or_open(doc_id).await?;
doc.set_sync_peer(remote_peer).await?;
```

### 4. Content Exchange (iroh-blobs)

Content-addressed data syncs naturally:

```rust
// Get content hash from remote
let hash = remote.get_blob_hash("refs/heads/main").await?;

// Download if we don't have it (deduped automatically)
if !local.has_blob(hash).await? {
    local.download_blob(hash, remote).await?;
}
```

## Example: How Forge Federates

Forge demonstrates the cluster-level layers + federation primitives pattern:

```
Cluster A (alice)                    Cluster B (bob)
+----------------------------+      +----------------------------+
| Directory: apps/forge->0x15|      | Directory: apps/forge->0x23|
|                            |      |                            |
| Forge Instance             |      | Forge Instance             |
| +- repos/aspen (local)     |      | +- repos/aspen (local)     |
| |  +- objects (blobs)      |      | |  +- objects (blobs)      |
| |  +- refs (KV)            |      | |  +- refs (KV)            |
| +- remotes/bob/aspen ------+------+-+                          |
|    (tracks bob's refs)     |      |                            |
+----------------------------+      +----------------------------+
```

### Federation Flow

1. **Alice runs Forge** on her cluster with cluster-local directory allocation
2. **Bob runs Forge** on his cluster with independent directory allocation
3. **Alice adds Bob as remote**: `git remote add bob aspen://bob-cluster/aspen`
4. **Forge discovers peer**: Uses peer discovery to find Bob's Forge instance
5. **Refs sync**: Alice's Forge fetches Bob's refs (app-level KV sync)
6. **Objects sync**: Git objects are content-addressed blobs (iroh-blobs handles transfer and dedup)

### Key Insight

The layers (Directory -> 0x15 on Alice, Directory -> 0x23 on Bob) are different.
The content (Git objects) is the same (content-addressed by hash).
Federation happens at the Forge application level, not the layer level.

## Implementation Phases

### Phase 1: Stabilize Cluster-Local Layers

- [ ] Extract `aspen-layer` crate from `aspen-core`
- [ ] Implement Directory Layer with high-contention allocator
- [ ] Document layer API for application developers
- [ ] Add comprehensive tests for layer operations

### Phase 2: Federation Primitives

- [ ] Peer discovery protocol (find clusters by app)
- [ ] App registry (advertise and query applications)
- [ ] Standard sync patterns (CRDT helpers)
- [ ] Content exchange patterns (blob sync helpers)

### Phase 3: Application Federation Patterns

- [ ] Document Forge federation as reference implementation
- [ ] Extract reusable federation patterns
- [ ] Create federation testing framework
- [ ] Write federation guide for app developers

## Design Decisions

### D1: Directory prefixes are cluster-local

The same path (e.g., `["apps", "forge"]`) may map to different binary prefixes on different clusters. This is intentional - clusters are sovereign.

Applications that need cross-cluster identity use content-addressing (hashes) or public keys, not paths.

### D2: No global namespace

There is no "global" directory or namespace. Each cluster has its own namespace. Applications coordinate via:

- Content hashes (same content = same hash)
- Public keys (identity across clusters)
- Application-specific protocols

### D3: Federation is opt-in

Clusters can operate in complete isolation. Federation is additive - connecting to other clusters adds capabilities but is never required for core functionality.

### D4: Applications own their federation logic

The framework provides primitives (discovery, sync, content exchange). Applications decide:

- What to sync
- When to sync
- How to resolve conflicts
- What consistency guarantees to provide

## Open Questions

1. **Standard app manifest format**: What metadata should apps expose for discovery?
2. **Federation authentication**: How do clusters authenticate to each other?
3. **Rate limiting**: How to prevent federation abuse?
4. **Conflict resolution**: Standard patterns for CRDT conflicts in apps?

## References

- FoundationDB Layer Concept: https://apple.github.io/foundationdb/layer-concept.html
- FoundationDB Directory Layer: https://apple.github.io/foundationdb/developer-guide.html#directories
- Iroh Documentation: https://iroh.computer/docs
- CRDTs: https://crdt.tech/
