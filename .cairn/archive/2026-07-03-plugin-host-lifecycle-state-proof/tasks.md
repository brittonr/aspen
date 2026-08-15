# Tasks: plugin-host-lifecycle-state-proof

## Phase 1: Lifecycle transition core

- [x] [serial] r[molten.plugin_lifecycle_state_proof.ordered_lifecycle] Define pure plugin lifecycle ordering checks for install, permission, activation, hostcall, health, upgrade, removal, and cleanup receipts.
- [x] [parallel] r[molten.plugin_lifecycle_state_proof.health_gate] Add health gate checks for continued activation, hostcall execution, and upgrade.
- [x] [parallel] r[molten.plugin_lifecycle_state_proof.cleanup_closes_authority] Add cleanup/removal checks that invalidate hostcall authority and callbacks.

## Phase 2: Tests

- [x] [parallel] r[molten.plugin_lifecycle_state_proof.ordered_lifecycle] Add a passing install→permission→hostcall→health→upgrade/remove trace.
- [x] [parallel] r[molten.plugin_lifecycle_state_proof.ordered_lifecycle] Add negative tests for hostcall before permission, undeclared hostcall, wrong ABI, stale supply-chain, and unauthorized namespace.
- [x] [parallel] r[molten.plugin_lifecycle_state_proof.health_gate] r[molten.plugin_lifecycle_state_proof.cleanup_closes_authority] Add failed-health, post-removal hostcall, and incomplete cleanup denial tests.

## Phase 3: Evidence and validation

- [x] [serial] r[molten.plugin_lifecycle_state_proof.ordered_lifecycle] r[molten.plugin_lifecycle_state_proof.health_gate] r[molten.plugin_lifecycle_state_proof.cleanup_closes_authority] Bind proof refs and run `cargo test plugin`.
