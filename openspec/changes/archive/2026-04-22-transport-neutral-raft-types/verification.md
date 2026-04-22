# Verification Evidence

This file tracks durable evidence for `transport-neutral-raft-types`.

## Implementation Evidence

- Changed file: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/.openspec.yaml`
- Changed file: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/proposal.md`
- Changed file: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/design.md`
- Changed file: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/tasks.md`
- Changed file: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/specs/transport/spec.md`
- Changed file: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/verification.md`
- Changed file: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`
- Changed file: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/warning-behavior.md`
- Changed file: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/openspec-preflight.txt`
- Changed file: `crates/aspen-raft-types/Cargo.toml`
- Changed file: `crates/aspen-raft-types/src/member.rs`
- Changed file: `crates/aspen-raft-types/src/request.rs`
- Changed file: `crates/aspen-raft/src/types.rs`
- Changed file: `crates/aspen-raft/src/network/factory.rs`
- Changed file: `crates/aspen-raft/src/network/tests.rs`
- Changed file: `crates/aspen-raft/src/node/kv_store.rs`
- Changed file: `crates/aspen-raft/src/node/membership_refresh.rs`
- Changed file: `crates/aspen-raft/src/node/mod.rs`
- Changed file: `crates/aspen-raft/src/node/trust.rs`
- Changed file: `crates/aspen-raft/src/storage_shared/trust.rs`
- Changed file: `crates/aspen-raft/src/write_batcher/direct_write.rs`
- Changed file: `crates/aspen-raft/src/write_batcher/flush.rs`
- Changed file: `crates/aspen-cluster/src/bootstrap/node/mod.rs`
- Changed file: `crates/aspen-cluster/src/relay_server.rs`
- Changed file: `crates/aspen-blob/src/replication/topology_watcher.rs`
- Changed file: `src/bin/aspen_node/main.rs`

## Task Coverage

- [x] I1 Replace `iroh::EndpointAddr` in `crates/aspen-raft-types` membership and trust payload types with `aspen_core::NodeAddress`, and update runtime callers to convert back to iroh only at shell boundaries. [covers=transport.transport-neutral-raft-membership-metadata,transport.transport-neutral-raft-membership-metadata.membership-metadata-stays-transport-neutral-at-rest]
  - Evidence: `crates/aspen-raft-types/Cargo.toml`, `crates/aspen-raft-types/src/member.rs`, `crates/aspen-raft-types/src/request.rs`, `crates/aspen-raft/src/types.rs`, `crates/aspen-raft/src/network/factory.rs`, `crates/aspen-raft/src/node/trust.rs`, `crates/aspen-raft/src/storage_shared/trust.rs`, `crates/aspen-cluster/src/bootstrap/node/mod.rs`

- [x] I2 Keep `RaftMemberInfo::default()` parseable for runtime helpers, and make invalid membership address conversion / endpoint-id parsing paths emit explicit warnings or deterministic forwarding failures instead of silently skipping the target. [covers=transport.transport-neutral-raft-membership-metadata.runtime-conversion-failure-is-surfaced-explicitly,transport.transport-neutral-raft-membership-metadata.default-member-metadata-stays-parseable-for-runtime-helpers]
  - Evidence: `crates/aspen-raft-types/src/member.rs`, `crates/aspen-raft/src/types.rs`, `crates/aspen-raft/src/network/tests.rs`, `crates/aspen-raft/src/node/kv_store.rs`, `crates/aspen-raft/src/node/membership_refresh.rs`, `crates/aspen-raft/src/node/mod.rs`, `crates/aspen-raft/src/write_batcher/direct_write.rs`, `crates/aspen-raft/src/write_batcher/flush.rs`, `crates/aspen-cluster/src/bootstrap/node/mod.rs`, `crates/aspen-cluster/src/relay_server.rs`, `src/bin/aspen_node/main.rs`

- [x] V1 Save the targeted cargo validation transcripts for this seam (`cargo test -p aspen-raft-types`, `cargo check -p aspen-raft`, `cargo check -p aspen-cluster`, `cargo check -p aspen-raft --features trust`, `cargo tree -p aspen-raft-types -e normal --depth 1`, `cargo test -p aspen-raft test_raft_member_info_default_endpoint_id_is_parseable --lib`, `cargo test -p aspen-raft test_member_endpoint_addr_rejects_invalid_node_address --lib`, `cargo test -p aspen-raft test_raft_member_info_construction --lib`, `cargo test -p aspen-raft --features trust test_collect_old_shares_accepts_valid_remote_share_from_stored_address --lib`, and `cargo test -p aspen-cluster --features relay-server test_extract_endpoint_ids_skips_invalid_member_endpoint_id --lib`), keep `verification.md` synchronized, and save `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/implementation-diff.txt` plus `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/openspec-preflight.txt`. [covers=transport.transport-neutral-raft-membership-metadata,transport.transport-neutral-raft-membership-metadata.membership-metadata-stays-transport-neutral-at-rest,transport.transport-neutral-raft-membership-metadata.runtime-conversion-failure-is-surfaced-explicitly,transport.transport-neutral-raft-membership-metadata.default-member-metadata-stays-parseable-for-runtime-helpers] [evidence=openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md]
  - Evidence: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`, `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/implementation-diff.txt`, `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/openspec-preflight.txt`

- [x] V2 Before archive, capture explicit warning-behavior coverage for invalid conversion paths that identifies the failing member and shows the operation returning its normal retryable/unavailable outcome. [covers=transport.transport-neutral-raft-membership-metadata.runtime-conversion-failure-is-surfaced-explicitly] [evidence=openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/warning-behavior.md]
  - Evidence: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/warning-behavior.md`

## Review Scope Snapshot

### `git diff HEAD -- crates/aspen-raft-types/Cargo.toml crates/aspen-raft-types/src/member.rs crates/aspen-raft-types/src/request.rs crates/aspen-raft/src/types.rs crates/aspen-raft/src/network/factory.rs crates/aspen-raft/src/network/tests.rs crates/aspen-raft/src/node/kv_store.rs crates/aspen-raft/src/node/membership_refresh.rs crates/aspen-raft/src/node/mod.rs crates/aspen-raft/src/node/trust.rs crates/aspen-raft/src/storage_shared/trust.rs crates/aspen-raft/src/write_batcher/direct_write.rs crates/aspen-raft/src/write_batcher/flush.rs crates/aspen-cluster/src/bootstrap/node/mod.rs crates/aspen-cluster/src/relay_server.rs crates/aspen-blob/src/replication/topology_watcher.rs src/bin/aspen_node/main.rs openspec/changes/archive/2026-04-22-transport-neutral-raft-types/tasks.md openspec/changes/archive/2026-04-22-transport-neutral-raft-types/verification.md`

- Status: captured after current edits
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/implementation-diff.txt`

## Verification Commands

### `cargo test -p aspen-raft-types`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`

### `cargo check -p aspen-raft`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`

### `cargo check -p aspen-cluster`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`

### `cargo check -p aspen-raft --features trust`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`

### `cargo test -p aspen-raft test_raft_member_info_default_endpoint_id_is_parseable --lib`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`

### `cargo test -p aspen-raft test_raft_member_info_construction --lib`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`

### `cargo test -p aspen-raft --features trust test_collect_old_shares_accepts_valid_remote_share_from_stored_address --lib`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`

### `cargo tree -p aspen-raft-types -e normal --depth 1`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`

### `cargo test -p aspen-raft test_member_endpoint_addr_rejects_invalid_node_address --lib`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`

### `cargo test -p aspen-cluster --features relay-server test_extract_endpoint_ids_skips_invalid_member_endpoint_id --lib`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/validation.md`

### `cargo test -p aspen-raft --features testing test_current_leader_info_warns_and_returns_none_for_invalid_leader_address --lib -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/warning-behavior.md`

### `scripts/openspec-preflight.sh transport-neutral-raft-types`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-22-transport-neutral-raft-types/evidence/openspec-preflight.txt`

## Notes

- Warning-behavior evidence now shows the failing member (`endpoint_id=not-a-public-key`) and the unavailable outcome (`leader_info=None`) for the invalid leader-address path.
