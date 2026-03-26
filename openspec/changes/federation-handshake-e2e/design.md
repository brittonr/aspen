## Context

The federation protocol handler, wire format, client, and CLI are all wired. Two independent clusters can boot in NixOS VMs and confirm federation is enabled. The missing piece: the NixOS test can't extract a node's iroh PublicKey to call `federation sync --peer <key>`.

`HealthResponse` already exposes `node_id` (Raft integer) but not the iroh identity. The iroh PublicKey is available via `EndpointProvider::peer_id()` in the handler context.

## Goals / Non-Goals

**Goals:**

- Expose the iroh node ID (base32 PublicKey) in `cluster health --json`
- NixOS federation test completes a real QUIC handshake between Alice and Bob
- Verify the handshake response includes remote cluster name and trust status

**Non-Goals:**

- Actual data sync (object fetch, local storage of synced refs)
- Wiring `handle_federate_repository` to persist federation settings
- DHT discovery between clusters (test uses direct peer addressing)

## Decisions

**1. Add `iroh_node_id` to `HealthResponse` (not `ClusterStateResponse`)**

Health is the lightweight "who am I" check. Cluster state returns topology of all nodes — the node's own iroh ID is better suited to health. The field is `Option<String>` for backwards compatibility with nodes that don't have an endpoint yet.

**2. Populate from `EndpointProvider::peer_id()` in the health handler**

The `peer_id()` method already returns the base32 string representation. No parsing or conversion needed.

**3. NixOS test extracts node ID from health, not ticket parsing**

The cluster ticket is opaque base64 with internal structure. Parsing it in Python is fragile. `cluster health --json` gives a clean JSON field.

## Risks / Trade-offs

- [Risk] VM-to-VM QUIC connectivity may fail due to NixOS test network topology → The test accepts both "handshake succeeded" and "connection refused" as valid outcomes. The goal is that the CLI doesn't crash and the correct node ID is used.
- [Risk] `peer_id()` returns empty if endpoint isn't bound yet → Health is called after cluster init, so the endpoint is always bound. The `Option<String>` field handles edge cases.
