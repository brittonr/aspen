# Tasks: raft-control-registry-determinism-proof

## Phase 1: Deterministic log application

- [ ] [serial] r[molten.consensus_state_machine_proof.registry_log_determinism] Add or strengthen bounded generated control-registry command logs applied to two independent runtimes.
- [ ] [parallel] r[molten.consensus_state_machine_proof.registry_log_determinism] Assert matching state refs, registry receipt refs, log entry refs, and commit receipt refs after each generated command.

## Phase 2: Replay and denial

- [ ] [parallel] r[molten.consensus_state_machine_proof.duplicate_client_sequence] Add positive replay tests proving duplicate client-session sequence returns prior result or denies without applying twice.
- [ ] [parallel] r[molten.consensus_state_machine_proof.duplicate_client_sequence] Add negative tests for conflicting duplicate command payloads, unsupported state-machine ids, malformed command schemas, and stale read evidence.

## Phase 3: Snapshot proof

- [ ] [serial] r[molten.consensus_state_machine_proof.snapshot_restore_equivalence] Add snapshot/restore equivalence tests for registry state refs and receipt bindings.
- [ ] [parallel] r[molten.consensus_state_machine_proof.snapshot_restore_equivalence] Add negative tests for tampered snapshot refs, missing snapshot content, and mismatched restore evidence.

## Phase 4: Validation

- [ ] [serial] r[molten.consensus_state_machine_proof.registry_log_determinism] r[molten.consensus_state_machine_proof.duplicate_client_sequence] r[molten.consensus_state_machine_proof.snapshot_restore_equivalence] Add traceability evidence and run `cargo test raft`.
