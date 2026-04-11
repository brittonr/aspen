# Verification Evidence

## Implementation Evidence

- Changed file: `crates/aspen-coordination/src/lockset.rs`
- Changed file: `crates/aspen-coordination/src/verified/lockset.rs`
- Changed file: `crates/aspen-coordination/src/spec/lockset_invariants.rs`
- Changed file: `crates/aspen-coordination/src/error.rs`
- Changed file: `crates/aspen-coordination/src/types.rs`
- Changed file: `crates/aspen-coordination/src/lib.rs`
- Changed file: `crates/aspen-coordination/src/spec/mod.rs`
- Changed file: `crates/aspen-coordination/src/verified/mod.rs`
- Changed file: `crates/aspen-coordination-protocol/src/lib.rs`
- Changed file: `crates/aspen-client-api/src/messages/coordination.rs`
- Changed file: `crates/aspen-client-api/src/messages/mod.rs`
- Changed file: `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs`
- Changed file: `crates/aspen-client/src/coordination/mod.rs`
- Changed file: `crates/aspen-client/src/coordination/lockset.rs`
- Changed file: `crates/aspen-core-essentials-handler/src/coordination.rs`
- Changed file: `crates/aspen-constants/src/coordination.rs`
- Changed file: `crates/aspen-constants/src/assertions.rs`
- Changed file: `crates/aspen-constants/src/lib.rs`
- Changed file: `crates/aspen-core/src/constants/mod.rs`
- Changed file: `openspec/changes/distributed-lockset/tasks.md`

## Task Coverage

- [x] 1.1 Add a bounded `LockSet` request / guard model in `crates/aspen-coordination` with canonical member normalization and per-resource fencing token tracking
  - Evidence: `crates/aspen-coordination/src/lockset.rs`, `crates/aspen-coordination/src/types.rs`, `crates/aspen-constants/src/coordination.rs`, `openspec/changes/distributed-lockset/evidence/implementation-diff.txt`
- [x] 1.2 Add pure helpers under `crates/aspen-coordination/src/verified/` for lock-set normalization, duplicate detection, per-member token computation, and set-wide backoff / renewal decisions
  - Evidence: `crates/aspen-coordination/src/verified/lockset.rs`, `crates/aspen-coordination/src/verified/mod.rs`, `openspec/changes/distributed-lockset/evidence/lockset-targeted-tests.txt`
- [x] 1.3 Add Verus coverage or equivalent proof-linked invariants for atomic all-or-nothing acquisition and per-resource fencing token monotonicity
  - Evidence: `crates/aspen-coordination/src/spec/lockset_invariants.rs`, `crates/aspen-coordination/src/spec/mod.rs`, `openspec/changes/distributed-lockset/evidence/lockset-targeted-tests.txt`
- [x] 2.1 Implement `try_acquire`, retrying `acquire`, `renew`, and `release` for distributed lock sets using one atomic conditional write per set operation
  - Evidence: `crates/aspen-coordination/src/lockset.rs`, `openspec/changes/distributed-lockset/evidence/cargo-check-coordination.txt`, `openspec/changes/distributed-lockset/evidence/lockset-targeted-tests.txt`
- [x] 2.2 Add bounded validation and actionable errors for duplicate members, oversized sets, and blocking live holders
  - Evidence: `crates/aspen-coordination/src/error.rs`, `crates/aspen-coordination/src/lockset.rs`, `crates/aspen-coordination/src/verified/lockset.rs`, `crates/aspen-client/src/coordination/lockset.rs`, `openspec/changes/distributed-lockset/tasks.md`, `openspec/changes/distributed-lockset/evidence/lockset-targeted-tests.txt`
- [x] 2.3 Add metrics and tracing for lock-set acquisition attempts, conflicts, renewals, releases, and takeover after expiry
  - Evidence: `crates/aspen-coordination/src/lockset.rs`, `crates/aspen-coordination/Cargo.toml`, `openspec/changes/distributed-lockset/evidence/cargo-check-coordination.txt`
- [x] 3.1 Add client API request / response types for lock-set acquire, try_acquire, renew, and release operations
  - Evidence: `crates/aspen-coordination-protocol/src/lib.rs`, `crates/aspen-client-api/src/messages/coordination.rs`, `crates/aspen-client-api/src/messages/mod.rs`, `crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs`, `openspec/changes/distributed-lockset/evidence/cargo-check-rpc-client.txt`
- [x] 3.2 Implement coordination RPC handlers and `aspen-client` wrappers for remote lock-set guards
  - Evidence: `crates/aspen-core-essentials-handler/src/coordination.rs`, `crates/aspen-client/src/coordination/lockset.rs`, `crates/aspen-client/src/coordination/mod.rs`, `openspec/changes/distributed-lockset/tasks.md`, `openspec/changes/distributed-lockset/evidence/lockset-targeted-tests.txt`
- [x] 3.3 Document canonical member ordering and guard semantics for downstream adopters in coordination-facing docs
  - Evidence: `crates/aspen-coordination/src/lib.rs`, `crates/aspen-client/src/coordination/mod.rs`, `openspec/changes/distributed-lockset/evidence/implementation-diff.txt`
- [x] 4.1 Add unit and property tests covering canonicalization, duplicate rejection, per-resource token monotonicity, and all-or-nothing contention behavior
  - Evidence: `crates/aspen-coordination/src/lockset.rs`, `crates/aspen-coordination/src/verified/lockset.rs`, `crates/aspen-coordination/src/spec/lockset_invariants.rs`, `openspec/changes/distributed-lockset/evidence/lockset-targeted-tests.txt`
- [x] 4.2 Add integration tests that exercise overlapping lock sets, expiry takeover, and renew / release through the remote client API
  - Evidence: `crates/aspen-core-essentials-handler/src/coordination.rs`, `crates/aspen-client/src/coordination/lockset.rs`, `crates/aspen-coordination/src/lockset.rs`, `openspec/changes/distributed-lockset/evidence/lockset-targeted-tests.txt`
- [x] 4.3 Demonstrate one higher-level adopter or end-to-end scenario using `LockSet` to coordinate a compound operation without partial acquisition
  - Evidence: `crates/aspen-core-essentials-handler/src/coordination.rs`, `crates/aspen-coordination/src/lib.rs`, `openspec/changes/distributed-lockset/evidence/lockset-targeted-tests.txt`

## Review Scope Snapshot

### `git diff HEAD -- crates/aspen-coordination/src/lockset.rs crates/aspen-coordination/src/verified/lockset.rs crates/aspen-coordination/src/spec/lockset_invariants.rs crates/aspen-coordination/src/error.rs crates/aspen-coordination/src/types.rs crates/aspen-coordination/src/lib.rs crates/aspen-coordination/src/spec/mod.rs crates/aspen-coordination/src/verified/mod.rs crates/aspen-coordination-protocol/src/lib.rs crates/aspen-client-api/src/messages/coordination.rs crates/aspen-client-api/src/messages/mod.rs crates/aspen-client-api/src/messages/to_operation/coordination_ops.rs crates/aspen-client/src/coordination/mod.rs crates/aspen-client/src/coordination/lockset.rs crates/aspen-core-essentials-handler/src/coordination.rs crates/aspen-constants/src/coordination.rs crates/aspen-constants/src/assertions.rs crates/aspen-constants/src/lib.rs crates/aspen-core/src/constants/mod.rs openspec/changes/distributed-lockset/tasks.md`

- Status: captured
- Artifact: `openspec/changes/distributed-lockset/evidence/implementation-diff.txt`

## Verification Commands

### `cargo check -p aspen-coordination`

- Status: pass
- Artifact: `openspec/changes/distributed-lockset/evidence/cargo-check-coordination.txt`

### `cargo check -p aspen-client-api -p aspen-core-essentials-handler -p aspen-client`

- Status: pass
- Artifact: `openspec/changes/distributed-lockset/evidence/cargo-check-rpc-client.txt`

### `cargo test -p aspen-coordination lockset --lib && cargo test -p aspen-client lockset && cargo test -p aspen-core-essentials-handler lockset`

- Status: pass
- Artifact: `openspec/changes/distributed-lockset/evidence/lockset-targeted-tests.txt`

### `scripts/openspec-preflight.sh distributed-lockset`

- Status: pass
- Artifact: `openspec/changes/distributed-lockset/evidence/preflight.txt`

## Notes

- A broader `cargo test -p aspen-coordination -p aspen-core-essentials-handler -p aspen-client` run still hits the pre-existing unrelated failure `crates/aspen-coordination/tests/proptest_model_based.rs::test_rate_limiter_burst_capacity`. Lock-set validation uses the targeted passing commands above.
- `openspec/changes/distributed-lockset/evidence/lockset-targeted-tests.txt` now includes the explicit-release regression `test_lockset_explicit_release_disarms_drop`, the blocked-details regression `test_lockset_client_try_acquire_exposes_blocked_details`, the non-contention regression `test_lockset_client_try_acquire_rejects_non_contention_errors`, and the partial-metadata regression `test_lockset_client_try_acquire_rejects_partial_contention_metadata`.
- `crates/aspen-client/src/coordination/lockset.rs` now treats `try_acquire` as blocked only when the response carries the full contention signal (`blocked_member` and `blocked_holder`). Partial metadata stays a hard error.
- `openspec/changes/distributed-lockset/tasks.md` was revalidated on 2026-04-11 for tasks 2.2 and 3.2 to reflect the corrected client behavior.
