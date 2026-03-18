# DAG Sync: Streaming Deterministic DAG Traversal for Aspen

## Why

Aspen's Forge sync (`SyncService`) fetches missing Git objects one at a time. Each missing object triggers a separate `download_from_peer` round trip. For a repository with 5,000 objects, that's 5,000 sequential requests — even though iroh QUIC could saturate the wire with a single stream.

The same problem applies to COB change sync, snix closure sync, and blob replication repair scans. Each has its own hand-rolled BFS loop with duplicated visited-set management, and none of them batch transfers.

The iroh-dag-sync experiment (from n0-computer/iroh-experiments) demonstrates a protocol where sender and receiver execute the same deterministic traversal over a single QUIC stream. The sender streams BAO-encoded objects in traversal order; the receiver walks the same traversal and inserts objects as they arrive. One round trip. Wire-saturating throughput.

This matters most for the dogfood use case — pushing the Aspen workspace to Forge should transfer at wire speed, not request-per-object speed.

## What Changes

- **New `aspen-dag` crate**: Composable DAG traversal trait and sync protocol, usable by any subsystem that walks content-addressed graphs.
- **Forge sync rewrite**: Replace per-object fetch loops with single-stream DAG sync for both Git objects and COB changes.
- **Stem/leaf split strategy**: Sync structural nodes (commits, trees, directories) first, then fan out leaf data (blobs, NARs) across multiple peers.
- **DAG sync ALPN**: New QUIC protocol for streaming DAG traversal, with inline predicates and traversal filters.

## Capabilities

### New Capabilities

- `dag-traversal`: Generic composable DAG traversal trait with depth-first, filtered, and sequenced modes.
- `dag-sync-protocol`: Single-stream QUIC protocol for deterministic DAG sync between two nodes.
- `stem-leaf-split`: Two-phase sync — structure first, data second — with multi-peer leaf download.
- `inline-predicate`: Sender-side decision on what data to inline vs. send as hash-only references.
- `partial-sync`: Receiver sends known heads so traversal short-circuits at already-synced boundaries.

### Modified Capabilities

- `forge-sync`: Forge's `SyncService` uses DAG sync instead of per-object fetch loops.
- `blob-replication`: Repair scans use DAG traversal for batch discovery of under-replicated object graphs.

## Impact

- **New crate**: `crates/aspen-dag/` (~1,500-2,500 lines)
- **Modified crates**: `aspen-forge` (sync.rs rewrite), `aspen-transport` (new ALPN), `aspen-blob` (traversal integration)
- **APIs**: New `DagTraversal` trait, `DagSyncRequest`/`DagSyncResponse` protocol types
- **Dependencies**: `bao-tree` for incremental verification framing (already in iroh-blobs dependency tree)
- **Testing**: Integration tests for single-stream sync, partial sync with visited sets, stem/leaf split, multi-peer leaf download. Madsim deterministic tests for traversal correctness.
