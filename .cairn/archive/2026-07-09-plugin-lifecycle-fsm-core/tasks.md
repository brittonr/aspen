# Tasks: plugin-lifecycle-fsm-core

- [x] [serial] r[molten.plugin_lifecycle_state_proof.transition_table] Define reviewed plugin lifecycle states, lifecycle events, transition relation, and guard classes in a pure core.
- [x] [serial] r[molten.plugin_lifecycle_state_proof.transition_table] Refactor lifecycle evaluation so activation, hostcall, upgrade, removal, cleanup, negotiation, compatibility, and recovery are evaluated as events over current state.
- [x] [parallel] r[molten.plugin_lifecycle_state_proof.guard_binding] Bind manifest, ABI, policy, resource, effect, supply-chain, extension, health, cleanup, and recovery evidence as explicit transition guards.
- [x] [parallel] r[molten.plugin_lifecycle_state_proof.state_receipts] Emit or extend lifecycle decision evidence to include prior state, event, target or next state, active manifest ref, selected guard refs, authority-closed flag, decision, and diagnostics.
- [x] [parallel] r[molten.plugin_lifecycle_state_proof.transition_tests] Add positive lifecycle progression fixtures and negative fixtures for hostcall-before-permission, stale manifest, failed-health upgrade, hostcall-after-removal, and incomplete cleanup.
- [x] [serial] r[molten.plugin_lifecycle_state_proof.transition_tests] Run focused plugin lifecycle tests and Cairn validation, then record evidence in implementation notes.