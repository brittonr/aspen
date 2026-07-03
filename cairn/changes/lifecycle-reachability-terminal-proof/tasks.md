# Tasks: lifecycle-reachability-terminal-proof

## Phase 1: Graph reachability

- [ ] [serial] r[molten.lifecycle_state_machine_proof.reachability] Add a pure lifecycle graph reachability helper or test-local computation over the allowed transition relation.
- [ ] [parallel] r[molten.lifecycle_state_machine_proof.reachability] Add positive tests for reachable lifecycle paths from `declared` through startup, degradation, stop, failure, restart, and cleanup paths.
- [ ] [parallel] r[molten.lifecycle_state_machine_proof.reachability] Add negative tests for forbidden shortcuts such as `declared -> ready` and `ready -> cleaned`.

## Phase 2: Terminal and cleanup boundaries

- [ ] [serial] r[molten.lifecycle_state_machine_proof.terminal_cleanup] Add tests proving `cleaned` has no outgoing passing transition.
- [ ] [parallel] r[molten.lifecycle_state_machine_proof.terminal_cleanup] Add tests proving `stopped` only proceeds to cleanup, `failed` only proceeds to restart or cleanup, and `restarting` only proceeds to starting or cleanup.

## Phase 3: Evidence

- [ ] [serial] r[molten.lifecycle_state_machine_proof.reachability] r[molten.lifecycle_state_machine_proof.terminal_cleanup] Add traceability evidence and run `cargo test lifecycle`.
