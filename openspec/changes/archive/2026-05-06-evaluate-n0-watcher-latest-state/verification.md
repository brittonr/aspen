# Verification: evaluate-n0-watcher-latest-state

## Implementation Evidence

- Changed file: `crates/aspen-blob/Cargo.toml`
- Changed file: `crates/aspen-blob/src/replication/manager.rs`
- Changed file: `crates/aspen-blob/src/replication/topology_watcher.rs`
- Changed file: `docs/patterns/latest-state-watchers.md`
- Changed file: `openspec/changes/evaluate-n0-watcher-latest-state/tasks.md`
- Changed file: `openspec/changes/evaluate-n0-watcher-latest-state/verification.md`
- Changed file: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/candidate-seam-inventory.md`
- Changed file: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/dependency-boundary.txt`
- Changed file: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/focused-watcher-tests.txt`

The selected seam is `aspen-blob` replication topology node-id observation. `LatestTopologyNodeIds` is a local prototype over `n0_watcher::Watchable<Vec<u64>>`; it normalizes node IDs and documents that slow observers may skip stale intermediate topology states. Durable/ordered streams remain rejected.

## Task Coverage

- [x] Create proposal, design, delta spec, and task rail for `evaluate-n0-watcher-latest-state`.
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/proposal.md`
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/design.md`
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/specs/latest-state-observation/spec.md`
- [x] Inspect existing Aspen watcher/broadcast usages and select one seam where latest-state semantics are correct.
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/candidate-seam-inventory.md`
- [x] Record rejected seams where every transition, event, or log item must remain durable/ordered.
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/candidate-seam-inventory.md`
  - Evidence: `docs/patterns/latest-state-watchers.md`
- [x] Add `n0-watcher` only to the selected crate or document why `tokio::sync::watch` remains better.
  - Evidence: `crates/aspen-blob/Cargo.toml`
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/dependency-boundary.txt`
- [x] Implement the prototype or no-adoption comparison note without changing durable/network protocols.
  - Evidence: `crates/aspen-blob/src/replication/topology_watcher.rs`
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/candidate-seam-inventory.md`
- [x] Add tests for initialization, latest-value convergence, slow-observer skipped values, and disconnect behavior.
  - Evidence: `crates/aspen-blob/src/replication/topology_watcher.rs`
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/focused-watcher-tests.txt`
- [x] Capture dependency-tree evidence that `n0-watcher` does not leak into alloc-only/core dependency paths.
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/dependency-boundary.txt`
- [x] Update docs/comments describing allowed latest-state observer use and forbidden durable-stream use.
  - Evidence: `docs/patterns/latest-state-watchers.md`
  - Evidence: `crates/aspen-blob/src/replication/topology_watcher.rs`
- [x] Run targeted tests, strict OpenSpec validation, helper verification, and `git diff --check`.
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/focused-watcher-tests.txt`
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/verification.md`
- [x] Sync/archive only after the prototype/adoption decision and all evidence tasks are complete.
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/tasks.md`
  - Evidence: `openspec/changes/evaluate-n0-watcher-latest-state/verification.md`

## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| watcher semantics | `cargo test -p aspen-blob --features replication latest_topology_node_ids -- --nocapture` | pass | `openspec/changes/evaluate-n0-watcher-latest-state/evidence/focused-watcher-tests.txt` | Covers initialization, latest convergence, slow-observer skip/convergence, and disconnect behavior for the selected seam. | Run full `cargo test -p aspen-blob --features replication` if broad replication churn follows. |
| dependency boundary | `cargo tree -p aspen-blob --features replication -i n0-watcher`; `cargo tree -p aspen-core --no-default-features -i n0-watcher` | pass | `openspec/changes/evaluate-n0-watcher-latest-state/evidence/dependency-boundary.txt` | Proves the direct dependency is local to `aspen-blob` and absent from protected `aspen-core --no-default-features`. | Re-run after any broader dependency promotion. |
| durable-stream rejection | Candidate seam inventory plus docs | pass | `openspec/changes/evaluate-n0-watcher-latest-state/evidence/candidate-seam-inventory.md`, `docs/patterns/latest-state-watchers.md` | Records accepted latest-state seam and rejects Raft/log/CI/Forge/hook/audit durable streams. | Add compile-time adapters only when a future spec names another seam. |
| whitespace | `git diff --check` | pass | `openspec/changes/evaluate-n0-watcher-latest-state/verification.md` | Ensures committed source/docs/evidence are whitespace-clean. | Re-run after archive. |

## Verification Commands

### `cargo test -p aspen-blob --features replication latest_topology_node_ids -- --nocapture`
- Status: pass
- Artifact: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/focused-watcher-tests.txt`

### `cargo tree -p aspen-blob --features replication -i n0-watcher`
- Status: pass
- Artifact: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/dependency-boundary.txt`

### `cargo tree -p aspen-core --no-default-features -i n0-watcher`
- Status: pass
- Artifact: `openspec/changes/evaluate-n0-watcher-latest-state/evidence/dependency-boundary.txt`

### `openspec validate evaluate-n0-watcher-latest-state --strict`
- Status: pass
- Artifact: `openspec/changes/evaluate-n0-watcher-latest-state/verification.md`

### `scripts/openspec-preflight.sh evaluate-n0-watcher-latest-state`
- Status: pass
- Artifact: `openspec/changes/evaluate-n0-watcher-latest-state/verification.md`

### `git diff --check`
- Status: pass
- Artifact: `openspec/changes/evaluate-n0-watcher-latest-state/verification.md`
