## 1. New crate: aspen-commit-dag

- [ ] 1.1 Scaffold `crates/aspen-commit-dag` with Cargo.toml, lib.rs, constants.rs, error.rs. Add to workspace members. Dependencies: `aspen-raft` (for ChainHash, GENESIS_HASH, compute_entry_hash), `aspen-kv-types`, `aspen-traits`, `aspen-constants`, `blake3`, `serde`, `dashmap`, `snafu`.
- [ ] 1.2 Define `CommitId` type alias (`pub type CommitId = ChainHash`) and `MutationType` enum (`Set(String)`, `Delete`) with serde derives.
- [ ] 1.3 Define `Commit` struct: `id: CommitId`, `parent: Option<CommitId>`, `branch_id: String`, `mutations: Vec<(String, MutationType)>`, `mutations_hash: ChainHash`, `raft_revision: u64`, `chain_hash_at_commit: ChainHash`, `timestamp_ms: u64`. Add serde derives and `postcard` serialization.
- [ ] 1.4 Define `DiffEntry` enum: `Added { key, value }`, `Removed { key }`, `Changed { key, old: MutationType, new: MutationType }`.
- [ ] 1.5 Define constants: `COMMIT_KV_PREFIX = "_sys:commit:"`, `COMMIT_TIP_PREFIX = "_sys:commit-tip:"`, `COMMIT_ORIGIN_PREFIX = "_sys:commit-origin:"`, `MAX_COMMIT_SNAPSHOT_KEYS = 10_000`, `MAX_COMMITS_PER_BRANCH = 10_000`, `COMMIT_GC_TTL_SECONDS = 604_800` (7 days), `COMMIT_GC_BATCH_SIZE = 1_000`, `COMMIT_GC_INTERVAL_SECS = 3_600`.
- [ ] 1.6 Define error types with snafu: `CommitNotFound`, `CommitCorrupted`, `CommitSerializationError`, `CommitStorageError`, `GcError`.

## 2. Verified commit hash functions

- [ ] 2.1 Create `src/verified/mod.rs` and `src/verified/commit_hash.rs`. Implement `compute_mutations_hash(sorted_mutations: &[(String, MutationType)]) -> ChainHash` — streaming BLAKE3 over sorted entries (key_len u32 LE, key bytes, tag byte, value_len u32 LE for Set, value bytes for Set).
- [ ] 2.2 Implement `compute_commit_id(parent: &Option<CommitId>, branch_id: &str, mutations_hash: &ChainHash, raft_revision: u64, timestamp_ms: u64) -> CommitId` — `blake3(parent_hash || branch_id_bytes || mutations_hash || raft_revision_le || timestamp_ms_le)`. Use `GENESIS_HASH` when parent is None.
- [ ] 2.3 Implement `verify_commit_integrity(commit: &Commit) -> bool` — recompute mutations_hash from commit.mutations and compare with constant_time_compare.
- [ ] 2.4 Write unit tests: determinism (same inputs → same output), different inputs → different hashes, empty mutations, tombstone-only mutations, sort order invariance.

## 3. Verus specifications

- [ ] 3.1 Create `verus/lib.rs` and `verus/commit_hash_spec.rs`. Define `compute_mutations_hash_spec` and `compute_commit_id_spec` as spec functions. Import `blake3_spec` and `u64_to_le_bytes` from `aspen-raft/verus/chain_hash_spec.rs`.
- [ ] 3.2 Prove determinism: same inputs → same CommitId.
- [ ] 3.3 Prove chain continuity: CommitId depends on parent (reuse `prev_hash_modification_detected` pattern from chain_verify_spec.rs).
- [ ] 3.4 Prove tamper detection: modified mutations → different mutations_hash (reuse `data_modification_detected` pattern).
- [ ] 3.5 Add `compute_commit_id` and `compute_mutations_hash` exec functions with ensures clauses linking to spec functions. Add helper functions (`is_valid_mutations_hash_size`, `compute_commit_hash_input_size`) matching the pattern in chain_hash_spec.rs.
- [ ] 3.6 Register in `scripts/verify-verus.sh` (or nix app) so `nix run .#verify-verus commit-dag` works.

## 4. Commit DAG storage

- [ ] 4.1 Implement `CommitStore` struct with methods: `store_commit(commit: &Commit, kv: &dyn KeyValueStore)` — serialize and write to `_sys:commit:{hex}`. `load_commit(id: &CommitId, kv: &dyn KeyValueStore) -> Result<Commit>` — read and deserialize. `update_branch_tip(branch_id: &str, commit_id: &CommitId, kv: &dyn KeyValueStore)` — write to `_sys:commit-tip:{branch_id}`.
- [ ] 4.2 Implement `get_branch_tip(branch_id: &str, kv: &dyn KeyValueStore) -> Result<Option<CommitId>>` — read from `_sys:commit-tip:{branch_id}` and parse hex.
- [ ] 4.3 Implement `walk_chain(start: CommitId, kv: &dyn KeyValueStore, max_depth: u32) -> Result<Vec<Commit>>` — follow parent links up to max_depth, returning commits in reverse chronological order.
- [ ] 4.4 Write integration tests with `DeterministicKeyValueStore`: store/load roundtrip, branch tip update, chain walk, nonexistent commit returns NotFound.

## 5. Diff

- [ ] 5.1 Implement `diff(a: &Commit, b: &Commit) -> Vec<DiffEntry>` as a pure function in `src/verified/diff.rs` — two-pointer merge over sorted mutations lists (same pattern as `merge_scan` in aspen-kv-branch).
- [ ] 5.2 Write unit tests: identical commits → empty diff, added keys, removed keys, changed values, tombstone transitions, mixed changes.
- [ ] 5.3 Add Verus spec `verus/diff_spec.rs`: prove output is sorted, prove no phantom entries (every DiffEntry corresponds to a real difference).

## 6. Fork from commit

- [ ] 6.1 Implement `fork_from(commit_id: CommitId, branch_id: String, parent_store: Arc<S>, kv: &dyn KeyValueStore) -> Result<BranchOverlay<S>>` in `aspen-commit-dag`. Loads commit, verifies integrity, creates BranchOverlay pre-populated with commit's mutations.
- [ ] 6.2 The new branch's internal parent_commit SHALL be set to the source CommitId so the next commit chains from it.
- [ ] 6.3 Write tests: fork from valid commit, fork from nonexistent commit, fork from corrupted commit (mutations_hash mismatch), dirty map matches commit mutations.

## 7. Garbage collection

- [ ] 7.1 Implement `CommitGc` struct with `run_gc(kv: &dyn KeyValueStore) -> Result<u64>` — scan `_sys:commit:` prefix, deserialize each, check TTL, check reachability (parent of non-expired commit or branch tip), delete expired unreachable commits in batches of `COMMIT_GC_BATCH_SIZE`.
- [ ] 7.2 Implement `spawn_gc_task(kv: Arc<dyn KeyValueStore>, interval: Duration) -> JoinHandle<()>` — periodic background task, skips on NOT_LEADER error (same pattern as `spawn_alert_evaluator`).
- [ ] 7.3 Write tests: expired commit with no refs is collected, expired commit with live child is protected, branch tip commit is never collected, GC batch size respected, GC on follower skips.

## 8. BranchOverlay integration

- [ ] 8.1 Add `commit-dag` feature to `aspen-kv-branch/Cargo.toml` with optional dep on `aspen-commit-dag`.
- [ ] 8.2 Add `parent_commit: Option<CommitId>` field to `BranchOverlay` (behind `#[cfg(feature = "commit-dag")]`). Default to None. Updated by `commit()` and `fork_from()`.
- [ ] 8.3 Add `CommitResult` struct: `write_result: WriteResult`, `commit_id: Option<CommitId>` (None when feature disabled).
- [ ] 8.4 Modify `commit()`: when `commit-dag` is enabled, snapshot dirty map (sorted), compute mutations_hash, compute CommitId, create Commit struct, add `_sys:commit:{hex}` and `_sys:commit-tip:{branch_id}` to the Raft batch, return CommitResult with CommitId. When disabled, return CommitResult with `commit_id: None`.
- [ ] 8.5 Ensure the commit metadata entries are included in the SAME Raft batch as data mutations (atomic — either all apply or none).
- [ ] 8.6 Update `commit_no_conflict_check()` with the same commit-dag logic.
- [ ] 8.7 Write tests: commit with feature enabled produces CommitId, second commit chains from first, CommitId is deterministic, commit without feature returns None, commit metadata is in KV after commit.

## 9. Federation: DocsExporter commit metadata

- [ ] 9.1 Add `commit-dag-federation` feature to `aspen-docs/Cargo.toml` with optional dep on `aspen-commit-dag`.
- [ ] 9.2 In `collect_payload_to_batch()`, commit metadata entries at `_sys:commit:` and `_sys:commit-tip:` already flow through as regular KV Set operations — verify this works with no code changes (the exporter processes all Set operations regardless of key prefix).
- [ ] 9.3 Write test: branch commit with commit-dag enabled → DocsExporter emits entries for data keys AND `_sys:commit:` AND `_sys:commit-tip:` keys.

## 10. Federation: DocsImporter commit verification

- [ ] 10.1 Add commit verification to `process_remote_entry()`: when `commit-dag-federation` is enabled and the key starts with `_sys:commit:`, deserialize the Commit and call `verify_commit_integrity()`. If verification fails, log warning and record `verified: false` in provenance. If it passes, record `verified: true`.
- [ ] 10.2 Implement provenance storage: on commit import, write `_sys:commit-origin:{commit_id_hex}` with source cluster ID, verification result, and import timestamp.
- [ ] 10.3 Implement commit entry buffering: when data entries arrive with no corresponding commit metadata yet, buffer up to `MAX_COMMIT_IMPORT_BUFFER` entries for `COMMIT_IMPORT_BUFFER_TIMEOUT` (30s). When the commit metadata arrives, verify and apply the batch atomically.
- [ ] 10.4 Implement per-peer rate limiting: track commits per peer per minute, drop excess beyond `MAX_COMMITS_PER_PEER_PER_MINUTE` (100).
- [ ] 10.5 Write tests: valid commit passes verification, tampered commit detected, missing parent buffered then applied, backward compatibility (entries without commit metadata applied normally), rate limit enforced.

## 11. Integration and wiring

- [ ] 11.1 Add `aspen-commit-dag` to workspace Cargo.toml, `fullSrc` in flake.nix, and any source filtering derivations.
- [ ] 11.2 Wire `CommitGc::spawn_gc_task()` into `aspen-node` main.rs (behind `commit-dag` feature), same pattern as `spawn_alert_evaluator()`.
- [ ] 11.3 Add CLI commands: `aspen-cli commit show <commit-id>`, `aspen-cli commit log <branch-id>` (walk chain), `aspen-cli commit diff <commit-a> <commit-b>`.
- [ ] 11.4 Run `cargo nextest run --workspace` — verify no regressions with feature off. Run with `--features commit-dag` — verify new tests pass.
- [ ] 11.5 Run `nix run .#verify-verus commit-dag` — verify all specs pass.
