# Tasks: node-profile-configuration

## Phase 1: Profile resolution core

- [x] [serial] r[molten.node_runtime.profile_backed_config] Add a pure profile-resolution core that converts checked exported profile inputs plus explicit override inputs into effective node config data.
- [x] [serial] r[molten.node_runtime.profile_override_policy] Define override classes and fail-closed denial diagnostics for invariant-weakening overrides.
- [x] [serial] r[molten.node_runtime.local_default_config_caveat] Mark no-profile node defaults as local-fixture configuration in the core result.

## Phase 2: Node lifecycle integration

- [x] [serial] r[molten.node_runtime.profile_backed_config] Add `molten node init` profile input handling while keeping filesystem reads in the CLI/daemon shell.
- [x] [serial] r[molten.node_runtime.profile_startup_receipt_binding] Bind effective profile metadata and selected refs into startup receipts for `run` and `serve`.
- [x] [serial] r[molten.node_runtime.profile_override_policy] Surface accepted and denied overrides in operator diagnostics and canonical receipts.

## Phase 3: Tests and validation

- [x] [parallel] r[molten.node_runtime.profile_backed_config] Add positive tests for profile-backed config construction and startup.
- [x] [parallel] r[molten.node_runtime.profile_startup_receipt_binding] Add negative tests for stale/tampered profile refs and unsupported metadata.
- [x] [parallel] r[molten.node_runtime.profile_override_policy] Add negative tests for release-invariant weakening overrides.
- [x] [parallel] r[molten.node_runtime.local_default_config_caveat] Add tests that no-profile fixture config cannot satisfy release profile evidence.
- [x] [serial] r[molten.node_runtime.profile_backed_config] Run focused node-runtime tests, formatting, and Cairn proposal/design/tasks/spec gates.
