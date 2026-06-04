## Phase 1: Canonical Raft/control-plane records

- [x] [serial] r[molten.raft_control_plane_registry.spec.control_scope] Define `raft-group-manifest-v1` with members, state-machine kind, command schemas, read mode, snapshot policy, policy, and resource refs.
- [x] [serial] r[molten.raft_control_plane_registry.spec.control_scope] Define `raft-command-envelope-v1`, `raft-log-entry-v1`, commit/read/snapshot/recovery receipt DTOs.
- [x] [parallel] r[molten.raft_control_plane_registry.spec.control_scope] Reject actor messages, gossip payloads, blob transfer, and ordinary choreography steps as Raft commands.
- [x] [parallel] r[molten.raft_control_plane_registry.spec.control_scope] Classify Raft/control registry artifacts in ledger/catalog views.

## Phase 2: Control registry state machine

- [x] [serial] r[molten.raft_control_plane_registry.spec.registry_apply] Implement pure deterministic registry apply/read/snapshot/restore for explicit control-plane namespaces.
- [x] [serial] r[molten.raft_control_plane_registry.spec.registry_apply] Add client session/sequence idempotency before apply.
- [x] [parallel] r[molten.raft_control_plane_registry.spec.registry_apply] Gate group install, proposals, membership changes, reads, and snapshots through policy/authority/resource evidence.
- [x] [parallel] r[molten.raft_control_plane_registry.spec.registry_apply] Emit registry apply/read/deny receipts binding command refs, state refs, and diagnostics.

## Phase 3: Trellis predicates and durability

- [x] [serial] r[molten.raft_control_plane_registry.spec.read_recovery] Wrap Trellis predicates for append consistency, quorum commit, and commit advancement.
- [x] [serial] r[molten.raft_control_plane_registry.spec.read_recovery] Implement read-index receipts and deny stale/unauthorized reads.
- [x] [parallel] r[molten.raft_control_plane_registry.spec.read_recovery] Add chunk-backed snapshot content refs and restore verification.
- [x] [parallel] r[molten.raft_control_plane_registry.spec.read_recovery] Persist log, snapshot, client sessions, and receipt indexes in a local durable store.

## Phase 4: Tests

- [x] [serial] r[molten.raft_control_plane_registry.spec.read_recovery] Add deterministic local cluster tests for proposal/commit/apply/read-index/recovery.
- [x] [serial] r[molten.raft_control_plane_registry.spec.registry_apply] Test protocol/artifact/policy pointer install/update/remove and receipt binding.
- [x] [parallel] r[molten.raft_control_plane_registry.spec.control_scope] Test ordinary actor-message rejection, duplicate client sequence, stale read, bad snapshot, and log gap denial.
- [x] [parallel] r[molten.raft_control_plane_registry.spec.registry_apply] Add Hegel properties for bounded logs, idempotency, snapshot roundtrip, and no-actor-traffic invariant.
