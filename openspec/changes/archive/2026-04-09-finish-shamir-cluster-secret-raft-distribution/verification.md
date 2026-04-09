# Verification Evidence

## Implementation Evidence

- Changed file: `crates/aspen-raft-types/src/lib.rs`
- Changed file: `crates/aspen-raft-types/src/request.rs`
- Changed file: `crates/aspen-raft/src/node/cluster_controller.rs`
- Changed file: `crates/aspen-raft/src/node/tests.rs`
- Changed file: `crates/aspen-raft/src/node/trust.rs`
- Changed file: `crates/aspen-raft/src/storage/in_memory/state_machine.rs`
- Changed file: `crates/aspen-raft/src/storage_shared/initialization.rs`
- Changed file: `crates/aspen-raft/src/storage_shared/log_storage.rs`
- Changed file: `crates/aspen-raft/src/storage_shared/mod.rs`
- Changed file: `crates/aspen-raft/src/storage_shared/state_machine/dispatch.rs`
- Changed file: `crates/aspen-raft/src/storage_shared/trust.rs`
- Changed file: `crates/aspen-raft/src/types.rs`
- Changed file: `crates/aspen-transport/src/log_subscriber/kv_operation.rs`
- Changed file: `tests/consumer_group_integration_test.rs`
- Changed file: `tests/forge_real_cluster_integration_test.rs`
- Changed file: `tests/fuse_raft_cluster_test.rs`
- Changed file: `tests/inmemory_api_test.rs`
- Changed file: `tests/multi_node_cluster_test.rs`
- Changed file: `tests/pubsub_integration_test.rs`
- Changed file: `tests/raft_node_direct_api_test.rs`
- Changed file: `tests/relay_status_test.rs`
- Changed file: `tests/support/bolero_generators.rs`
- Changed file: `tests/types_proptest.rs`
- Changed file: `tests/verify_integration_test.rs`
- Changed file: `tests/write_batching_integration.rs`
- Changed file: `openspec/specs/cluster-secret-lifecycle/spec.md`
- Changed file: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/proposal.md`
- Changed file: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/design.md`
- Changed file: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/specs/cluster-secret-lifecycle/spec.md`
- Changed file: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/tasks.md`
- Changed file: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/verification.md`

## Task Coverage

- [x] 1.1 Add a `TrustInitialize` application request payload to `crates/aspen-raft-types` for epoch, per-node share bytes, and epoch digests.
  - Evidence: `crates/aspen-raft-types/src/request.rs`, `crates/aspen-raft-types/src/lib.rs`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-types-tests.txt`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/implementation.diff`

- [x] 1.2 Teach both redb and in-memory state-machine dispatch paths to handle `TrustInitialize` requests.
  - Evidence: `crates/aspen-raft/src/storage_shared/state_machine/dispatch.rs`, `crates/aspen-raft/src/storage/in_memory/state_machine.rs`, `crates/aspen-raft/src/storage_shared/log_storage.rs`, `crates/aspen-raft/src/storage_shared/trust.rs`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-trust-tests.txt`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/implementation.diff`

- [x] 1.3 Track the local node ID inside `SharedRedbStorage` so trust-init apply can select only the local share.
  - Evidence: `crates/aspen-raft/src/storage_shared/mod.rs`, `crates/aspen-raft/src/storage_shared/initialization.rs`, `crates/aspen-raft/src/storage_shared/trust.rs`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-trust-tests.txt`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/implementation.diff`

- [x] 2.1 Change `RaftNode::initialize_trust()` to submit the committed `TrustInitialize` request through `raft.client_write()` instead of writing leader-local storage directly.
  - Evidence: `crates/aspen-raft/src/node/trust.rs`, `crates/aspen-raft/src/node/cluster_controller.rs`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/implementation.diff`

- [x] 2.2 Update the cluster-init control-plane flow to await the async trust initialization hook and surface trust-init write failures cleanly.
  - Evidence: `crates/aspen-raft/src/node/cluster_controller.rs`, `crates/aspen-raft/src/node/trust.rs`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-no-feature-build.txt`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/implementation.diff`

- [x] 3.1 Add a state-machine test proving `TrustInitialize` stores only the local share while persisting shared digests.
  - Evidence: `crates/aspen-raft/src/storage_shared/trust.rs`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-trust-tests.txt`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/implementation.diff`

- [x] 3.2 Add a node-level regression test covering multi-node trust init request construction and threshold validation.
  - Evidence: `crates/aspen-raft/src/node/trust.rs`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-trust-tests.txt`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/implementation.diff`

- [x] 3.3 Run targeted trust-enabled tests for `aspen-raft` and `aspen-trust`.
  - Evidence: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-types-tests.txt`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-trust-tests.txt`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-no-feature-build.txt`

- [x] 3.4 Add a multi-node cluster-init integration test that proves follower share persistence after the committed trust-init request is applied.
  - Evidence: `crates/aspen-raft/src/node/tests.rs`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-trust-multinode-test.txt`, `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/implementation.diff`

## Review Scope Snapshot

### `git diff -- crates/aspen-raft-types/src/lib.rs crates/aspen-raft-types/src/request.rs crates/aspen-raft/src/node/cluster_controller.rs crates/aspen-raft/src/node/tests.rs crates/aspen-raft/src/node/trust.rs crates/aspen-raft/src/storage/in_memory/state_machine.rs crates/aspen-raft/src/storage_shared/initialization.rs crates/aspen-raft/src/storage_shared/log_storage.rs crates/aspen-raft/src/storage_shared/mod.rs crates/aspen-raft/src/storage_shared/state_machine/dispatch.rs crates/aspen-raft/src/storage_shared/trust.rs crates/aspen-raft/src/types.rs crates/aspen-transport/src/log_subscriber/kv_operation.rs tests/consumer_group_integration_test.rs tests/forge_real_cluster_integration_test.rs tests/fuse_raft_cluster_test.rs tests/inmemory_api_test.rs tests/multi_node_cluster_test.rs tests/pubsub_integration_test.rs tests/raft_node_direct_api_test.rs tests/relay_status_test.rs tests/support/bolero_generators.rs tests/types_proptest.rs tests/verify_integration_test.rs tests/write_batching_integration.rs openspec/specs/cluster-secret-lifecycle/spec.md openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/proposal.md openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/design.md openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/tasks.md openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/specs/cluster-secret-lifecycle/spec.md`

- Status: captured
- Artifact: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/implementation.diff`

## Verification Commands

### `cargo test -p aspen-raft-types`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-types-tests.txt`

### `cargo test -p aspen-raft --features trust 'trust' -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-trust-tests.txt`

### `cargo test -p aspen-raft --features trust,testing test_multi_node_trust_init_persists_follower_shares -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-trust-multinode-test.txt`

### `cargo test -p aspen-raft --no-run`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/aspen-raft-no-feature-build.txt`

### `scripts/openspec-preflight.sh openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-09-finish-shamir-cluster-secret-raft-distribution/evidence/openspec-preflight.txt`

## Notes

- `crates/aspen-raft/src/node/tests.rs` is compiled only with `feature = "testing"`, so the multi-node follower-persistence regression test is verified with `--features trust,testing`.
- `openspec archive` could not merge the delta spec automatically because the existing main spec still used delta-style sections, so `openspec/specs/cluster-secret-lifecycle/spec.md` was updated manually before archive.
- The root integration and property tests needed mechanical updates for the new `InitRequest.trust` field and `AppRequest::TrustInitialize` variant so workspace clippy stayed green.
