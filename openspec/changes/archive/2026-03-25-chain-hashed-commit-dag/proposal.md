## Why

Aspen has three systems that need to be connected: chain hashing (Raft log integrity, Verus-verified), BranchOverlay (CoW KV branching with atomic commit), and federation sync (iroh-docs export/import of KV state). Today these operate independently. Chain hashes protect the Raft log but don't extend to the KV layer. BranchOverlay commits are anonymous batches with no identity or history. Federation sync exports bare key-value pairs with no provenance, atomicity, or integrity verification. A compromised or buggy federated peer can send fabricated KV entries through an authenticated iroh connection and the importing cluster has no way to detect it.

Chain-hashed commits unify these systems. Every BranchOverlay commit produces a CommitId (BLAKE3 hash chained to its parent), forming an append-only DAG. Commits capture the mutations, the Raft revision they landed at, and the chain hash at that point. Federation sync carries commit metadata alongside KV data, enabling the importing cluster to verify that received state was produced by legitimate Raft consensus. The same DAG enables deterministic replay and diff between any two points in branch history.

## What Changes

- New `CommitId` type aliasing `ChainHash` (`[u8; 32]`), reusing the existing BLAKE3 chain hash infrastructure from `aspen-raft/src/verified/integrity.rs`
- New `Commit` struct stored immutably in KV at `_sys:commit:{hex}` — contains parent link, branch ID, mutation snapshot, Raft revision, chain hash binding, and timestamp
- `BranchOverlay.commit()` extended to compute a chain hash over the dirty map, store the `Commit`, and return a `CommitId`
- New `BranchOverlay.fork_from(CommitId)` to create a branch pre-populated from a historical commit's mutation snapshot
- New `diff(CommitId, CommitId)` function comparing two commits' mutation snapshots
- `DocsExporter` extended to include commit metadata (CommitId, parent, batch boundaries) alongside KV entries
- `DocsImporter` extended to verify commit chain integrity on import before applying entries
- Commit DAG garbage collection via TTL or reachability-based pruning
- Verus specs proving commit DAG inherits the same integrity properties as Raft log chain hashing (tamper detection, rollback detection, divergence propagation)

## Capabilities

### New Capabilities

- `commit-dag`: Core CommitId type, Commit struct, DAG storage, chain hash computation, fork-from-commit, diff, garbage collection
- `commit-dag-federation`: Federation-aware commit metadata in DocsExporter/DocsImporter — batch atomicity, provenance verification, chain integrity checking on import

### Modified Capabilities

- `kv-branch-overlay`: BranchOverlay.commit() returns CommitId, tracks parent commit, supports fork_from(CommitId)
- `federation`: DocsExporter carries commit metadata, DocsImporter verifies chain integrity

## Impact

- **Crates modified**: `aspen-kv-branch` (CommitId, Commit, extended commit), `aspen-docs` (exporter/importer commit metadata), `aspen-raft` (reuse verified integrity types)
- **New crate**: `aspen-commit-dag` (CommitId, Commit, DAG storage, GC, diff) — keeps `aspen-kv-branch` focused on the overlay mechanics
- **KV storage**: New `_sys:commit:{hex}` prefix for commit metadata, `_sys:commit-tip:{branch_id}` for branch head pointers
- **Wire format**: DocsExporter batch entries gain optional commit metadata fields — backwards compatible (old importers ignore unknown fields)
- **Verus**: New specs in `aspen-commit-dag/verus/` reusing chain hash axioms from `aspen-raft/verus/chain_hash_spec.rs`
- **Resource bounds**: MAX_COMMIT_SNAPSHOT_KEYS, MAX_COMMIT_DAG_DEPTH, COMMIT_GC_TTL_SECONDS, MAX_COMMITS_PER_BRANCH
