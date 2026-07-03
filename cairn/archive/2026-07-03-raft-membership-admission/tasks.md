# Tasks: raft-membership-admission

## Phase 1: Membership model

- [x] [serial] r[molten.raft_membership_admission.model] Define canonical membership-change request, preflight receipt, and eventual commit receipt records with group, target, role, configuration, evidence, and diagnostics bindings.
- [x] [serial] r[molten.raft_membership_admission.stronger_than_peer] Enforce that connected peer sessions, transport observations, and topic joins cannot satisfy Raft/control-plane membership admission.
- [x] [parallel] r[molten.raft_membership_admission.peer_boundary] Update peer-bootstrap boundaries to require a separate membership command and receipt for any Raft voter or non-voter control-plane role.

## Phase 2: Admission core

- [x] [serial] r[molten.raft_membership_admission.preflight_checks] Implement pure membership preflight checks for peer session scope, authority, policy, resource, source-gate/provenance, state-machine compatibility, snapshot/replay readiness, and operator evidence.
- [x] [serial] r[molten.raft_membership_admission.quorum_safety] Bind Trellis/Raft predicate receipts for quorum preservation and configuration transition safety before any membership commit can pass.
- [x] [parallel] r[molten.raft_membership_admission.diagnostics] Add diagnostics that distinguish peer connectivity, membership preflight, committed membership state, and linearizable read evidence.

## Phase 3: CLI and tests

- [x] [serial] r[molten.raft_membership_admission.cli_preflight] Add an operator dry-run preflight command and readback summary before implementing or enabling mutating membership changes.
- [x] [serial] r[molten.raft_membership_admission.positive_negative_tests] Add positive preflight fixtures and negative tests for connected-peer-only, missing authority, missing source-gate, incompatible state-machine, stale snapshot, revoked peer, and quorum-safety denial.

## Phase 4: Validation

- [x] [serial] r[molten.raft_membership_admission.validation] Run focused membership tests, consensus/peer-bootstrap tests, formatting, and Cairn validation before archiving.
