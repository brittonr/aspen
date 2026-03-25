## Context

Aspen has three independent integrity/replication systems:

1. **Chain hashing** (`aspen-raft/src/verified/integrity.rs`): BLAKE3 chain hash on every Raft log entry. Background `ChainVerifier` detects corruption. Verus-verified specs prove tamper detection, rollback detection, and divergence propagation. Operates at the Raft log layer only.

2. **BranchOverlay** (`aspen-kv-branch`): CoW overlay on `KeyValueStore` with dirty map, tombstones, read-set tracking, and atomic commit via `WriteCommand::Batch` or `OptimisticTransaction`. Commits are anonymous — they produce a `WriteResult` with a `header_revision` but no identity, no history, no chain linkage.

3. **Federation sync** (`aspen-docs`): `DocsExporter` subscribes to the Raft log broadcast and exports individual `(key, value)` pairs to iroh-docs CRDT namespaces. `DocsImporter` receives entries via range-based set reconciliation and applies them to the local KV store with priority-based conflict resolution. No batch atomicity, no provenance, no integrity verification on import.

The gap: chain hashing stops at the Raft log. Nothing above it — KV branches, federation — carries integrity guarantees.

## Goals / Non-Goals

**Goals:**

- Every `BranchOverlay.commit()` produces a `CommitId` (BLAKE3 chain hash) linking it to its parent commit, forming an append-only DAG
- Commits are immutable snapshots stored in KV, enabling fork-from-commit and diff
- Federation sync carries commit metadata so importing clusters can verify that received KV state was produced by legitimate Raft consensus
- Commit DAG inherits the same Verus-verified integrity properties as the Raft log chain (tamper detection, rollback detection, divergence propagation)
- Resource-bounded: GC prevents unbounded DAG growth, commit snapshots are size-limited

**Non-Goals:**

- Automatic commit on every KV mutation (too expensive, wrong granularity — explicit commit is the right model)
- Full KV state snapshots in commits (commits store only the dirty map mutations, not the entire KV state at that point)
- Replacing the Raft log chain hash (the commit DAG is a higher-level chain, not a replacement)
- VM-level snapshots or process memory forking (Cloud Hypervisor snapshot/restore is a separate concern)
- Cross-cluster commit DAG merging (importing clusters verify and store commits, they don't splice DAGs together)

## Decisions

### 1. CommitId is a ChainHash alias, not a UUID

**Decision**: `type CommitId = ChainHash` (`[u8; 32]`), computed as `blake3(parent_commit_hash || branch_id || mutations_hash || raft_revision || timestamp_ms)`.

**Alternative considered**: UUIDv7 (what ix.dev uses). UUIDv7 gives time-ordering but no integrity — any node can fabricate a UUID. Chain hashing gives both ordering (via the DAG) and integrity (via cryptographic linkage).

**Rationale**: The chain hash infrastructure already exists, is Verus-verified, and provides the exact properties needed. Reusing `ChainHash` means the same constant-time comparison, same hex encoding, same 32-byte fixed size.

### 2. Mutations stored as sorted Vec, not full KV snapshot

**Decision**: A `Commit` stores `mutations: Vec<(String, MutationType)>` where `MutationType` is `Set(String)` or `Delete`. This is the dirty map at commit time, sorted by key.

**Alternative considered**: Storing the full KV state at each commit (enables point-in-time queries without replay). Too expensive — a 10,000-key dirty map is 10,000 entries per commit. Full state would be the entire KV store.

**Rationale**: The dirty map is bounded by `MAX_BRANCH_DIRTY_KEYS` (10,000) and `MAX_BRANCH_TOTAL_BYTES` (64MB). Storing just mutations keeps commits small. Reconstructing state at a point requires walking the DAG — acceptable for audit/replay use cases that are infrequent.

### 3. Mutations hash uses merkle-style BLAKE3

**Decision**: `mutations_hash = blake3(sorted_key_value_pairs)` — iterate sorted mutations, hash each `(key, mutation_type, value_bytes)` tuple into a streaming BLAKE3 hasher.

**Alternative considered**: Full merkle tree. More complex, enables partial verification, but overkill — commits are small (bounded by branch limits) and verified atomically, not piecemeal.

**Rationale**: Streaming BLAKE3 over sorted entries is simple, deterministic, and fast (~3GB/s). The sort order makes the hash deterministic regardless of insertion order. This goes in `src/verified/` as a pure function.

### 4. New crate `aspen-commit-dag`, not extension of `aspen-kv-branch`

**Decision**: `CommitId`, `Commit`, DAG storage, GC, diff, and fork-from live in a new `aspen-commit-dag` crate. `aspen-kv-branch` gains an optional dependency on it.

**Alternative considered**: Adding everything to `aspen-kv-branch`. Mixes overlay mechanics (CoW dirty map, scan merge) with DAG semantics (history, diff, GC).

**Rationale**: Separation of concerns. `aspen-kv-branch` stays focused on the `KeyValueStore` overlay. `aspen-commit-dag` handles history, identity, and integrity. The integration point is `BranchOverlay.commit()` calling into `aspen-commit-dag` to create a `Commit` when the feature is enabled.

### 5. Feature-gated integration

**Decision**: `aspen-kv-branch` gains a `commit-dag` feature that enables the `aspen-commit-dag` dependency. Without it, `commit()` works exactly as today (returns `WriteResult`, no commit metadata). With it, `commit()` additionally stores a `Commit` and returns a `CommitId`.

**Rationale**: Zero overhead for users who don't need commit history. Federation sync enables commit metadata export via a separate `commit-dag-federation` feature on `aspen-docs`.

### 6. Federation commit metadata as sideband entries

**Decision**: `DocsExporter` exports commit metadata as special KV entries at `_sys:commit:{hex}` alongside normal data entries. The importer recognizes these by prefix and verifies the chain before applying the associated data entries.

**Alternative considered**: Extending the `BatchEntry` struct with commit fields. Breaks the `DocsWriter` trait contract and requires all writer implementations to handle commit metadata.

**Rationale**: KV entries at a system prefix (`_sys:commit:`) flow through the existing export/import pipeline unchanged. The importer already filters by prefix (subscription filters). Commit metadata entries are just more KV pairs — no protocol changes needed. The importer adds a verification step before applying entries that reference a commit.

### 7. GC via TTL with reachability protection

**Decision**: Commits get a configurable TTL (default: 7 days). GC runs periodically, scanning `_sys:commit:` entries and deleting expired ones. Commits referenced as `parent` by a non-expired commit are protected from GC regardless of their own TTL.

**Alternative considered**: Reference counting. Complex, error-prone with concurrent branch operations, requires atomic decrement on branch delete.

**Alternative considered**: No GC, manual pruning. Leads to unbounded growth.

**Rationale**: TTL-based GC is simple, bounded, and matches the existing KV TTL infrastructure. Reachability protection prevents breaking active commit chains. A branch's head commit is always live (stored at `_sys:commit-tip:{branch_id}`), which transitively protects its ancestors within the TTL window.

### 8. Genesis commit per branch

**Decision**: Each branch has a genesis commit (CommitId = BLAKE3 of branch creation metadata, parent = None). The first `commit()` on a branch chains from this genesis.

**Rationale**: Mirrors the Raft log's `GENESIS_HASH = [0u8; 32]` pattern. Every chain needs a root. The genesis commit captures when and why the branch was created, and gives fork-from-commit a stable starting point.

## Risks / Trade-offs

**[Storage overhead from commit metadata]** → Each commit stores mutations plus metadata in KV. A branch with 5,000 dirty keys produces a ~200KB commit entry. Mitigated by: TTL-based GC, `MAX_COMMITS_PER_BRANCH` limit, mutations are already bounded by `MAX_BRANCH_DIRTY_KEYS`.

**[GC race with fork-from-commit]** → A commit could be GC'd between when a user reads its CommitId and when they call `fork_from()`. Mitigated by: reachability protection (if a branch tip points to it, it's live), and returning a clear error ("commit not found, may have been garbage collected") rather than silent failure.

**[Federation import verification cost]** → Verifying the commit chain on import adds latency. Each commit verification is one BLAKE3 hash computation (~nanoseconds) plus one KV read for the parent. Mitigated by: verification is per-commit not per-key, and commits batch many keys. A 1000-key batch adds one hash check, not 1000.

**[Backwards compatibility of DocsExporter]** → Old importers that don't understand commit metadata will see `_sys:commit:` entries as regular KV writes and store them. Harmless — they're just KV entries with a system prefix. The data entries still arrive and apply normally. Mitigated by: commit metadata is additive, not required for basic sync.

**[Commit chain divergence after branch rebase]** → If a branch is rebased (dirty map rewritten before commit), the chain hash changes. This is correct behavior — the chain accurately reflects that a different set of mutations was committed. Not a bug, but users need to understand that CommitIds are content-dependent.

## Future: Memory Layout for RDMA and Zero-Copy

The current design stores commits as serialized KV entries (postcard → redb via Raft). This is correct for the iroh QUIC transport. If Aspen adds a datacenter-local transport tier (RDMA via libfabric, shared memory between co-located processes), the memory layout of commits and chain structures becomes performance-critical. These patterns should be evaluated at that time:

### Arena/Slab pattern for in-memory commit DAGs

Replace `HashMap<CommitId, Commit>` with a flat `Vec<CommitSlot>` where parent links are `u32` indices instead of `CommitId` lookups. Eliminates pointer-chasing and keeps the working set contiguous for CPU cache. Relevant for GC reachability analysis when the commit store grows to tens of thousands of entries.

```rust
struct CommitSlot {
    id: CommitId,
    parent_idx: Option<u32>,  // index into arena, not CommitId
    timestamp_ms: u64,
    is_tip: bool,
}

// Contiguous, cache-friendly, serializable as a single buffer
let arena: Vec<CommitSlot> = load_commits_as_arena(&kv_store).await?;
```

**When to adopt**: GC scan profiling shows cache misses on parent-link traversal, or commit store exceeds ~100K entries.

### Epoch-Based Reclamation for concurrent chain access

If commit objects are held in memory (hot cache) and accessed concurrently by multiple threads without the KV store as intermediary, `crossbeam-epoch` provides lock-free reads with deferred reclamation. Threads "pin" an epoch during reads; deleted nodes are freed only after all pinned threads advance. Faster than RwLock for read-heavy workloads.

**When to adopt**: Hot commit cache with concurrent readers (e.g., multiple federation importers verifying chains simultaneously on the same node).

### Zero-copy deserialization with rkyv

`rkyv` enables reading commit fields directly from a byte buffer (mmap'd file, network buffer, RDMA region) without allocation. Chain parent links become relative offsets within the buffer rather than absolute pointers. Enables verifying `mutations_hash` without deserializing the full `Commit`.

```rust
// With rkyv, verify directly from the byte buffer
let archived = rkyv::check_archived_root::<Commit>(buffer)?;
let recomputed = compute_mutations_hash(&archived.mutations);
constant_time_compare(&recomputed, &archived.mutations_hash)
```

**When to adopt**: RDMA transport added, or mmap-backed commit store for datacenter deployments where serialization overhead matters.

### Transport-aware serialization

The current postcard format is compact and fast but not zero-copy. If RDMA support is added, consider a dual serialization strategy: postcard for iroh QUIC (compact wire format), rkyv for RDMA/shared-memory (zero-copy, relative offsets). The `Commit` struct can derive both `serde::Serialize` and `rkyv::Archive` simultaneously.

| Technique | Complexity | When to Use |
|---|---|---|
| `Vec` + indices (arena) | Low | GC reachability over large commit stores |
| `crossbeam-epoch` | Medium | Hot commit cache with concurrent readers |
| `rkyv` zero-copy | High | RDMA transport, mmap-backed storage |
| Dual serialization | Medium | Mixed transport (QUIC + RDMA) deployments |

**Reference**: ix.dev uses libfabric for TCP-to-RDMA transport switching without application rewrites. If Aspen adds a similar abstraction, the commit DAG's memory layout should be the first candidate for the zero-copy treatment since commit verification is on the hot path for federation import.
