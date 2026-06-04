# Tasks: Node Control Authority Delegation

## Phase 1: Canonical delegation evidence

- [x] [serial] r[molten.node_control_authority_delegation.spec.grant_artifacts] Add canonical node-control authority grant artifacts.
- [x] [serial] r[molten.node_control_authority_delegation.spec.live_pre_enqueue_gate] Add canonical authority decision receipts for live ingress.

## Phase 2: Live ingress gate

- [x] [serial] r[molten.node_control_authority_delegation.spec.live_pre_enqueue_gate] Gate live ingress before enqueue on admitted delegation evidence.
- [x] [serial] r[molten.node_control_authority_delegation.spec.fail_closed] Fail closed for unknown grant, wrong peer, wrong operation, wrong target/resource scope, expired grant, and revoked grant.
- [x] [serial] r[molten.node_control_authority_delegation.spec.transport_non_authority] Keep transport identity, peer bootstrap, policy/resource refs, and provenance gates separate from delegation authority.

## Phase 3: CLI and validation

- [x] [parallel] r[molten.node_control_authority_delegation.spec.grant_artifacts] Add CLI fixture helper for authority grant creation/import.
- [x] [parallel] r[molten.node_control_authority_delegation.spec.fail_closed] Add unit and CLI coverage for live delegation admission and denial paths.
