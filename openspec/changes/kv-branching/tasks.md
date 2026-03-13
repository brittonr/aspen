## 1. Crate Scaffolding and Constants

- [ ] 1.1 Create `crates/aspen-kv-branch/` with `Cargo.toml` — deps: `aspen-traits`, `aspen-kv-types`, `aspen-constants`, `dashmap`, `serde`, `tracing`, `tokio`
- [ ] 1.2 Add `kv-branch` feature flag to workspace `Cargo.toml` gating `dep:aspen-kv-branch`
- [ ] 1.3 Add `kv-branch` to the `full` feature set
- [ ] 1.4 Define Tiger Style constants in `src/constants.rs`: `MAX_BRANCH_DIRTY_KEYS = 10_000`, `MAX_BRANCH_TOTAL_BYTES = 64 MB`, `MAX_BRANCH_DEPTH = 8`, `BRANCH_COMMIT_TIMEOUT_MS = 10_000`

## 2. Core BranchOverlay Type

- [ ] 2.1 Create `src/entry.rs` with `BranchEntry` enum: `Write { value: String }` and `Tombstone`
- [ ] 2.2 Create `src/overlay.rs` with `BranchOverlay<S: KeyValueStore>` struct: `branch_id: String`, `parent: Arc<S>`, `dirty: DashMap<String, BranchEntry>`, `read_set: DashMap<String, i64>` (key → mod_revision), `dirty_bytes: AtomicU64`, `depth: u8`
- [ ] 2.3 Implement `BranchOverlay::new(branch_id, parent, depth)` with depth check against `MAX_BRANCH_DEPTH`
- [ ] 2.4 Implement `BranchOverlay::child(branch_id)` that creates a nested branch with `depth + 1`

## 3. KeyValueStore Trait Implementation

- [ ] 3.1 Implement `read()`: check dirty map (return Write value or NotFound for Tombstone), fall through to `parent.read()`, record `(key, mod_revision)` in read set
- [ ] 3.2 Implement `write()`: check `MAX_BRANCH_DIRTY_KEYS` and `MAX_BRANCH_TOTAL_BYTES` bounds, insert `BranchEntry::Write` into dirty map, update `dirty_bytes`
- [ ] 3.3 Implement `delete()`: insert `BranchEntry::Tombstone` into dirty map, adjust `dirty_bytes` if replacing a Write entry
- [ ] 3.4 Implement `scan()`: call parent scan, call `merge_scan()` verified function with dirty entries + parent results + prefix + limit, return merged result
- [ ] 3.5 Write unit tests: read fall-through, read delta, read tombstone, write buffering, delete tombstone, scan merge basic cases

## 4. Verified Scan Merge Logic

- [ ] 4.1 Create `src/verified/mod.rs` and `src/verified/scan_merge.rs` with pure `merge_scan()` function: takes sorted branch entries, sorted parent entries, prefix, limit → returns merged sorted vec
- [ ] 4.2 Implement prefix filtering: only include branch entries whose key starts with scan prefix
- [ ] 4.3 Implement tombstone filtering: exclude entries where the branch has a Tombstone
- [ ] 4.4 Implement sorted merge: two-pointer merge of branch and parent iterators, branch takes precedence on duplicate keys
- [ ] 4.5 Implement limit enforcement: stop after `limit` entries
- [ ] 4.6 Write property tests with proptest: tombstoned keys never appear, output is sorted, branch overrides parent, limit is respected

## 5. Verus Specifications for Scan Merge

- [ ] 5.1 Create `verus/lib.rs` and `verus/scan_merge_spec.rs` with state model for merge inputs/outputs
- [ ] 5.2 Write `spec fn` for tombstone exclusion: for all keys in output, no tombstone exists in branch
- [ ] 5.3 Write `spec fn` for sort order: for all adjacent pairs in output, `output[i].key < output[i+1].key`
- [ ] 5.4 Write `spec fn` for branch precedence: if key in both branch and parent, output value == branch value
- [ ] 5.5 Write `exec fn merge_scan` with `ensures` clauses referencing all three spec functions
- [ ] 5.6 Run `nix run .#verify-verus` and fix any proof failures

## 6. Commit and Abort

- [ ] 6.1 Implement `commit()` for root-level branches (depth=0 or parent is not a BranchOverlay): collect dirty map into `Vec<WriteOp>`, check `MAX_BATCH_SIZE` bound
- [ ] 6.2 Implement commit without read set: emit `WriteCommand::Batch { operations }` to parent
- [ ] 6.3 Implement commit with read set: emit `WriteCommand::OptimisticTransaction { read_set, write_set }` to parent
- [ ] 6.4 Implement commit timeout: wrap Raft write in `tokio::time::timeout(BRANCH_COMMIT_TIMEOUT)`
- [ ] 6.5 Implement `commit()` for nested branches: merge dirty map + tombstones into parent overlay's dirty map, merge read sets
- [ ] 6.6 Implement `Drop` for `BranchOverlay`: log branch abort if dirty map is non-empty (debug tracing only)
- [ ] 6.7 Write tests: commit with empty read set, commit with conflict detection (success and rejection), nested commit merges into parent, drop discards writes, commit exceeding MAX_BATCH_SIZE returns error

## 7. Branch Configuration

- [ ] 7.1 Create `src/config.rs` with `BranchConfig` struct: optional overrides for `max_dirty_keys`, `max_total_bytes`, `commit_timeout`
- [ ] 7.2 Add `BranchOverlay::with_config(branch_id, parent, config)` constructor
- [ ] 7.3 Add `BranchOverlay::stats()` returning `BranchStats { dirty_count, dirty_bytes, read_set_size, depth }`
- [ ] 7.4 Write tests: custom limits, stats reporting

## 8. FUSE @branch Virtual Paths

- [ ] 8.1 Add `branches: DashMap<String, Arc<BranchOverlay<SharedClient>>>` field to `AspenFs`
- [ ] 8.2 Modify `path_to_key()` to detect `@branch-name` as first component, return `(branch_name, rest_of_path)` or `(None, full_path)`
- [ ] 8.3 Route `lookup`, `read`, `write`, `readdir`, `create`, `unlink` through branch overlay when `@branch` prefix is present
- [ ] 8.4 Implement `.branchfs_ctl` virtual file: handle writes of "commit", "abort", "create:{name}" via `write()` FUSE handler
- [ ] 8.5 Implement root `readdir` extension: include `@branch-name` entries for all active branches
- [ ] 8.6 Implement branch `readdir`: merge branch dirty entries with parent directory listing (reuse verified scan merge)
- [ ] 8.7 Drop all branches on `destroy()` (FUSE unmount)
- [ ] 8.8 Write tests: read/write through @branch path, commit/abort via ctl file, create via ctl file, readdir shows branches, unmount drops branches

## 9. Saga Executor Integration

- [ ] 9.1 Add `branch_backed: bool` field to `SagaStep` in `aspen-jobs/src/saga/types.rs` (default false)
- [ ] 9.2 Add `run_step_in_branch()` method to `SagaExecutor`: creates `BranchOverlay`, passes it as the KV store to the step function
- [ ] 9.3 Implement atomic commit+journal: on step success, collect branch dirty ops + saga state Set into one `WriteCommand::Batch`
- [ ] 9.4 On step failure: drop branch, call `fail_step()` — skip compensation for the branched step
- [ ] 9.5 Wire `get_next_action()` to recognize branch-backed steps: set `requires_compensation = false` when `branch_backed = true` and step failed
- [ ] 9.6 Write tests: branch-backed step success (atomic commit + journal), branch-backed step failure (no orphaned keys, no compensation), mixed saga with branch and compensation steps, crash recovery replays branched step

## 10. CI Job Integration

- [ ] 10.1 Modify `aspen-ci-executor-vm` lifecycle to create `BranchOverlay` wrapping the KV store before passing to `AspenFs::with_prefix`
- [ ] 10.2 On job success: commit the branch after VirtioFS daemon shutdown
- [ ] 10.3 On job failure: drop the branch (existing cleanup logic can be simplified/removed)
- [ ] 10.4 Feature-gate CI integration behind `#[cfg(feature = "kv-branch")]`
- [ ] 10.5 Write integration test: CI job writes workspace keys, fails, verify no keys in base store
- [ ] 10.6 Write integration test: CI job writes workspace keys, succeeds, verify keys in base store

## 11. Deploy Executor Integration

- [ ] 11.1 Modify deploy executor to run deploy writes inside a `BranchOverlay`
- [ ] 11.2 On health check pass: commit the branch
- [ ] 11.3 On health check fail: drop the branch, base state unchanged
- [ ] 11.4 Feature-gate behind `#[cfg(feature = "kv-branch")]`
- [ ] 11.5 Write test: deploy with passing health check commits, deploy with failing health check leaves base unchanged

## 12. Documentation and Feature Wiring

- [ ] 12.1 Add `aspen-kv-branch` to workspace members in root `Cargo.toml`
- [ ] 12.2 Wire `kv-branch` feature into `aspen-node` binary features
- [ ] 12.3 Add module docs to `aspen-kv-branch/src/lib.rs` with usage examples
- [ ] 12.4 Add entry to `docs/` with branching model overview, durable execution composition, and resource bounds
