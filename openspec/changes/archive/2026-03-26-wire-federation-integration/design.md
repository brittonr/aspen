## Context

All federation primitives exist in `aspen-federation` (9,500 lines): `ClusterIdentity`, `TrustManager`, `FederationDiscoveryService`, `FederationGossipService`, `FederationProtocolHandler`, `SyncOrchestrator`, `FederationResourceResolver` trait. The sync protocol implements handshake, resource listing, object sync, and ref verification over QUIC with ALPN `/aspen/federation/1`.

None of these run during normal node operation. The `RouterBuilder` has methods for raft, client, blobs, gossip, dag-sync, castore, nix-cache, nostr, and HTTP proxy — but not federation. Bootstrap creates no federation services. The `FederationProtocolHandler` is only used in test code.

The node startup flow is:

1. `bootstrap_node()` in `aspen-cluster/src/bootstrap/node/mod.rs` creates storage, network, discovery, hooks, workers, blobs, sync
2. `src/node/mod.rs` `spawn_router_with_blobs()` registers ALPN handlers on the iroh Router
3. Missing: federation init step and federation ALPN registration

## Goals / Non-Goals

**Goals:**

- Federation services start automatically when configured (cluster key present)
- Incoming federation sync requests are served via the ALPN handler
- Forge repos can be resolved through `FederationResourceResolver` using real KV/blob data
- NixOS VM test proves end-to-end: repo create → federate → cross-cluster sync → verify

**Non-Goals:**

- DHT announcements (requires `global-discovery` feature, not needed for direct peer sync)
- Gossip service wiring (useful but orthogonal — can sync without gossip)
- Automatic background sync scheduling (first pass: explicit pull only)
- CLI federation commands beyond what exists (status, trust, list-federated already work)

## Decisions

### D1: Federation init as a bootstrap sub-module

Add `federation_init.rs` alongside the existing `blob_init.rs`, `discovery_init.rs`, etc. in `crates/aspen-cluster/src/bootstrap/node/`. This module reads `NodeConfig.federation` and creates:

- `ClusterIdentity` (from config secret key or generate+persist)
- `TrustManager` (pre-populated with configured trusted clusters)
- `FederationProtocolContext` (wired to the cluster's endpoint and HLC)
- `FederationProtocolHandler` (passed back to the caller for Router registration)

Alternative: wire everything in `src/node/mod.rs` directly. Rejected — that file is already 856 lines and the bootstrap sub-module pattern is established.

### D2: RouterBuilder gets a `federation()` method

Follow the existing pattern: `pub fn federation<F: ProtocolHandler>(mut self, handler: F) -> Self`. Uses `FEDERATION_ALPN` from `aspen-federation`. Feature-gated behind `cfg(feature = "federation")`.

Alternative: register directly in `spawn_router_with_blobs`. Rejected — breaks the RouterBuilder abstraction that every other protocol uses.

### D3: ForgeResourceResolver implements FederationResourceResolver

Create a `ForgeResourceResolver` in `aspen-forge` (or `aspen-forge-handler`) that:

- Maps `FederatedId` → repo by scanning federation settings stored in KV
- `get_resource_state()` returns current ref heads from the forge repo's KV keys
- `sync_objects()` returns git objects as blobs from the forge repo

This is the first concrete resolver. The trait is already generic enough for other apps (CI artifacts, docs, etc.) to add their own resolvers later.

Alternative: implement resolver at the `aspen-rpc-handlers` level. Rejected — the resolver needs forge-specific knowledge (repo layout, ref format).

### D4: Federation config in NodeConfig

Add to `crates/aspen-cluster/src/config/federation.rs` (already exists):

- `cluster_secret_key: Option<String>` — hex Ed25519 secret key
- `cluster_name: Option<String>` — human-readable name
- `trusted_clusters: Vec<String>` — hex public keys of trusted peers

When `cluster_secret_key` is `None`, federation is disabled and no handler is registered.

### D5: NixOS test uses direct iroh connection, no DHT

The 2-cluster VM test already has both clusters running. To test actual sync:

1. Cluster A creates a repo, writes a ref, federates it
2. Use the sync client (`connect_to_cluster` + `get_remote_resource_state`) from cluster B
3. Verify returned refs match what cluster A has

This requires exposing the sync client through the CLI or a test binary. Simplest path: add a `federation sync` CLI command that performs a one-shot sync pull from a specified peer.

## Risks / Trade-offs

- **[Risk] ForgeResourceResolver couples federation to forge** → Mitigation: resolver is behind a trait, forge impl is optional (feature-gated). Other apps add their own.
- **[Risk] Config key management adds operational complexity** → Mitigation: auto-generate if not provided, persist to data dir. Zero-config for single-cluster (federation stays disabled).
- **[Risk] `spawn_router_with_blobs` grows more complex** → Mitigation: federation registration is a single `.federation(handler)` call, same as every other protocol.
- **[Risk] NixOS test may be slow with 2 full clusters** → Mitigation: existing test already boots 2 clusters successfully; we add assertions to the existing flow.
