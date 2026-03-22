# DAG Sync — Design

## Context

Aspen stores content-addressed data (Git objects, COB changes, snix directories, blobs) across the cluster. These objects form DAGs — a commit points to a tree, which points to blobs; a COB change points to parent changes; a snix PathInfo points to a Directory, which points to sub-directories and blobs.

Today each subsystem implements its own graph walk and per-object fetch logic. Forge's `SyncService` does BFS with individual `download_from_peer` calls. Snix's `RaftDirectoryService` has a recursive traversal with a hardcoded depth limit. Blob replication scans individual hashes. All three patterns are structurally identical: walk a DAG, find missing nodes, fetch them.

The iroh-dag-sync experiment demonstrates that a single deterministic traversal over one QUIC stream can replace all these per-object fetch patterns, with wire-saturating throughput and a single round trip.

## Goals / Non-Goals

**Goals:**

- Single `DagTraversal` trait usable by Forge, snix, and blob replication
- Single-stream DAG sync protocol that replaces per-object fetching
- Stem/leaf split for two-phase sync with multi-peer parallelism
- Bounded resource usage (Tiger Style limits on depth, transfer size, request size)
- Verified pure functions for traversal state computation

**Non-Goals:**

- Multi-hash support (Aspen is BLAKE3-only, no IPFS interop needed)
- Generic traversal language (project-specific traversals in Rust, per iroh-dag-sync's recommendation)
- Replacing iroh-blobs' own transfer protocol (DAG sync uses it for individual blob fallback)
- CRDT/iroh-docs integration (orthogonal, handled by existing `aspen-docs`)

## Decisions

### 1. New `aspen-dag` Crate

**Choice:** Extract DAG traversal and sync protocol into a standalone `crates/aspen-dag/` crate.

**Rationale:** Forge, snix, and blob replication all need the same traversal primitives. A shared crate avoids three copies of the same logic. The traversal trait is generic over hash type and DB accessor, so it composes with different storage backends.

**Alternative:** Inline traversal logic in each consumer crate. Rejected because the three existing implementations already duplicate the same BFS/DFS + visited-set + fetch-missing pattern.

**Implementation:**

```
crates/aspen-dag/
├── src/
│   ├── lib.rs              # Public API
│   ├── traversal.rs        # DagTraversal trait + FullTraversal, SequenceTraversal
│   ├── filter.rs           # Composable filters (Filtered, codec-based, size-based)
│   ├── protocol.rs         # DagSyncRequest, DagSyncResponse, frame types
│   ├── sync.rs             # handle_sync_request, handle_sync_response
│   ├── constants.rs        # Tiger Style bounds
│   └── verified/           # Pure traversal state functions
│       ├── mod.rs
│       └── traversal.rs    # should_visit, compute_next, is_traversal_bounded
├── verus/
│   ├── lib.rs
│   └── traversal_spec.rs   # Verus specs for traversal invariants
└── Cargo.toml
```

### 2. Trait-Based DAG Traversal

**Choice:** A `DagTraversal` trait generic over hash type, with mutable DB access between steps.

**Rationale:** iroh-dag-sync's `Traversal` trait design is correct — the receiver must be able to insert newly arrived data between traversal steps so the next step can discover children of the just-inserted node. The trait must give `&mut self` access to the DB.

**Alternative:** Iterator-based traversal that pre-computes the full sequence. Rejected because the receiver doesn't have the data to compute the sequence — it builds incrementally as objects arrive.

**Implementation:**

```rust
pub trait DagTraversal {
    type Hash: Copy + Eq + Hash;
    type Db;
    type Error;

    /// Advance to the next node in the traversal.
    /// Returns None when the traversal is complete.
    fn next(&mut self) -> impl Future<Output = Result<Option<Self::Hash>, Self::Error>>;

    /// Mutable access to the database for inserting received data.
    fn db_mut(&mut self) -> &mut Self::Db;

    /// The root hashes of this traversal.
    fn roots(&self) -> Vec<Self::Hash>;
}
```

Composable combinators:

- `Filtered<T, F>`: Skip nodes matching a predicate
- `Bounded<T>`: Stop after N nodes or N depth
- `Sequenced<T>`: Traverse a fixed sequence of hashes

### 3. Link Extraction as a Trait

**Choice:** A `LinkExtractor` trait that given a hash and its data, returns child hashes.

**Rationale:** Different DAG types have different link structures. Git commits link to trees and parents. Git trees link to blobs and subtrees. COB changes link to parent changes. Snix directories link to sub-directories and blobs. The traversal doesn't need to know the format — it just needs child hashes.

**Alternative:** Hard-code link extraction per DAG type. Rejected because the traversal logic is identical across all types; only link extraction differs.

**Implementation:**

```rust
pub trait LinkExtractor {
    type Hash: Copy + Eq + Hash;
    type Error;

    /// Extract child hashes from a node's data.
    fn extract_links(&self, hash: &Self::Hash, data: &[u8]) -> Result<Vec<Self::Hash>, Self::Error>;
}
```

Forge provides `GitLinkExtractor` (understands commits, trees, tags, blobs). Snix provides `DirectoryLinkExtractor`. Generic blob replication can use a `NoLinks` extractor for flat hash sets.

### 4. Wire Protocol Design

**Choice:** Single QUIC bidirectional stream. Postcard-encoded request. Sequence of fixed-size header frames (33 bytes: 1 discriminator + 32 hash) followed by optional BAO4-encoded data.

**Rationale:** Matches iroh-dag-sync's proven design. Fixed-size headers avoid framing ambiguity. BAO4 encoding gives 16 KiB incremental verification. Postcard is already used throughout Aspen for wire encoding.

**Alternative:** gRPC streaming. Rejected — Aspen is iroh-only, no HTTP.

**Alternative:** One QUIC stream per object. Rejected — defeats the purpose of batched transfer.

**Frame format:**

```
Response = Frame*

Frame = HashOnly | DataInline

HashOnly:
  discriminator: u8 = 0x00
  blake3_hash:   [u8; 32]

DataInline:
  discriminator: u8 = 0x01
  blake3_hash:   [u8; 32]
  size:          u64 (little-endian)
  data:          BAO4-encoded chunks (verified per 16 KiB group)
```

### 5. Stem/Leaf Split via Traversal Filters

**Choice:** Implement stem/leaf as composable traversal filters, not as a separate protocol.

**Rationale:** iroh-dag-sync shows this works — `NoRaw` filter for stem, `JustRaw` for leaves. Same traversal trait, different filter predicate. No new protocol machinery needed.

**Implementation:** Forge defines node type classification:

```rust
pub enum ForgeNodeType { Commit, Tree, Blob, Tag }

pub fn stem_filter(hash: &Hash, node_type: ForgeNodeType) -> bool {
    matches!(node_type, ForgeNodeType::Commit | ForgeNodeType::Tree | ForgeNodeType::Tag)
}

pub fn leaf_filter(hash: &Hash, node_type: ForgeNodeType) -> bool {
    matches!(node_type, ForgeNodeType::Blob)
}
```

Multi-peer leaf download: after stem sync completes, the receiver has all blob hashes. It partitions them across available peers (e.g., by hash modulo peer count) and issues parallel `Sequence` traversals — one per peer.

### 6. Visited Set for Partial Sync

**Choice:** Include optional `known_heads` in `DagSyncRequest`. Sender terminates traversal when encountering a known head.

**Rationale:** For incremental Git syncs, the receiver knows its local branch tips. Sending these upfront lets the sender skip the entire already-synced subgraph. For a repo with 10,000 commits where 50 are new, this skips traversal of 9,950 commits.

**Alternative:** Bloom filter of known hashes. More compact but has false positives — could skip objects the receiver doesn't actually have. Rejected for correctness.

**Implementation:** `DagSyncRequest` includes `known_heads: Option<BTreeSet<Hash>>`. The `FullTraversal` checks each node against this set before descending.

## Risks / Trade-offs

**[Traversal divergence]** If sender and receiver traverse differently (different code versions, different link extraction), the stream becomes garbage. Mitigation: version the ALPN (`DAG_SYNC/1`), include traversal method in the request, abort on unexpected hashes.

**[Large visited sets]** A repository with millions of objects could produce a huge visited set. Mitigation: Tiger Style bound (`MAX_VISITED_SET_SIZE`). For very large repos, use `known_heads` (just branch tips) instead of full visited sets.

**[Slow peers block the stream]** Single-stream means one slow object blocks all subsequent objects. Mitigation: stem/leaf split reduces the critical path. The stem (small metadata) completes fast; leaves (large data) are parallelized across peers.

**[Memory pressure on receiver]** The receiver must hold traversal state (stack + visited set) in memory during sync. Mitigation: bounded traversal depth (`MAX_DAG_TRAVERSAL_DEPTH = 10,000`), bounded visited set, and the receiver can abort at any time.
