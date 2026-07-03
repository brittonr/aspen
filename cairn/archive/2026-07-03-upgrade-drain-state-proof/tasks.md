# Tasks: upgrade-drain-state-proof

## Phase 1: Drain readiness predicate

- [x] [serial] r[molten.upgrade_drain_state_proof.terminal_protocol_gate] Define a pure drain readiness check over task refs, affected protocol refs, protocol gate receipts, and terminal state refs.
- [x] [parallel] r[molten.upgrade_drain_state_proof.protocol_ref_binding] Add exact from/to/affected/compatibility ref binding checks.
- [x] [parallel] r[molten.upgrade_drain_state_proof.no_mutation_on_deny] Add before/after state-ref capture for drain denial paths.

## Phase 2: Tests

- [x] [parallel] r[molten.upgrade_drain_state_proof.terminal_protocol_gate] Add a passing protocol-drain task with non-empty terminal gate evidence.
- [x] [parallel] r[molten.upgrade_drain_state_proof.terminal_protocol_gate] r[molten.upgrade_drain_state_proof.protocol_ref_binding] Add negative tests for missing gate, denied gate, wrong protocol, stale compatibility ref, and empty terminal refs.
- [x] [parallel] r[molten.upgrade_drain_state_proof.no_mutation_on_deny] Add no-mutation assertions for denied drain and cutover paths.

## Phase 3: Evidence and validation

- [x] [serial] r[molten.upgrade_drain_state_proof.terminal_protocol_gate] r[molten.upgrade_drain_state_proof.protocol_ref_binding] r[molten.upgrade_drain_state_proof.no_mutation_on_deny] Bind proof refs and run `cargo test upgrade protocol rewrite`.
