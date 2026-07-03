## ADDED Requirements

### Requirement: Control-registry command logs are deterministic
r[molten.consensus_state_machine_proof.registry_log_determinism] Molten MUST prove that independent control-registry runtimes applying the same admitted command log produce identical state refs, registry receipt refs, log entry refs, and commit receipt refs after each committed command.

#### Scenario: Matching logs converge
- GIVEN two independent control-registry runtimes with the same manifest
- WHEN both apply the same bounded admitted command log
- THEN their registry state refs match after every committed command
- AND their emitted registry and commit receipt refs match.

### Requirement: Duplicate client sequences do not apply twice
r[molten.consensus_state_machine_proof.duplicate_client_sequence] Molten MUST prove that duplicate client-session and sequence-number commands return prior result evidence or deny without applying the state-machine mutation a second time.

#### Scenario: Duplicate command preserves state
- GIVEN a committed control-registry command for a client session and sequence number
- WHEN the same client session and sequence number is submitted again
- THEN Molten returns prior result evidence or a denial receipt
- AND the registry state ref does not advance a second time.

### Requirement: Control-registry snapshots restore equivalent state
r[molten.consensus_state_machine_proof.snapshot_restore_equivalence] Molten MUST prove that control-registry snapshots and restore receipts preserve canonical registry state refs and fail closed for missing, stale, or tampered snapshot evidence.

#### Scenario: Snapshot restore preserves registry ref
- GIVEN a control-registry runtime with committed commands and a canonical snapshot
- WHEN Molten restores a fresh runtime from the snapshot
- THEN the restored registry state ref equals the snapshotted state ref
- AND restore evidence binds the snapshot ref and checks.
