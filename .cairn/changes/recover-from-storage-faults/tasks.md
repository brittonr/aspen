# Tasks: Recover from storage faults

## Baseline and contracts

- [ ] [serial] Record current durability labels, Redb commit paths, snapshot ordering, repair behavior, consensus caveats, and focused test baselines. r[molten.fabric_durability.storage_fault_outcomes] r[molten.consensus.storage_fault_recovery]
- [ ] [serial] Define typed persistence observations, uncertain outcomes, poison reasons, and recovery states in the pure core. r[molten.fabric_durability.storage_fault_outcomes] r[molten.fabric_durability.adapter_quarantine]
- [ ] [parallel] Add positive and negative pure classification tests for every observation and invalid transition. r[molten.fabric_durability.storage_fault_validation]

## Persistence shell

- [ ] [serial] Refactor Redb shells to acknowledge only observed durability and to poison the adapter after uncertain or inconsistent results. r[molten.fabric_durability.storage_fault_outcomes] r[molten.fabric_durability.adapter_quarantine]
- [ ] [serial] Add explicit reopen and read-back reconciliation without retrying a failed synchronization as proof of durability. r[molten.fabric_durability.adapter_quarantine]
- [ ] [serial] Publish snapshot payloads through a pinned Durable File Publication adapter before metadata commit. r[molten.fabric_durability.snapshot_publication]
- [ ] [parallel] Add commit, synchronization, corruption, partial-publication, parent-sync, reopen, and identity-mismatch tests. r[molten.fabric_durability.storage_fault_validation]

## Consensus recovery

- [ ] [serial] Add pure present, missing, faulty, committed, uncommitted, and unknown repair decisions keyed by term and index. r[molten.consensus.storage_fault_recovery]
- [ ] [serial] Add peer-query and snapshot-repair effect plans while retaining networking, persistence, and time in the shell. r[molten.consensus.storage_fault_recovery]
- [ ] [serial] Add participant-scoped recovery progress for local sufficiency, peer-available repair, complete global absence, incomplete observation, and finite virtual horizons. r[molten.consensus.storage_fault_recovery_progress]
- [ ] [serial] Remove voting readiness after repair or persistent-state uncertainty and require full recovery admission before rejoin. r[molten.consensus.repair_admission]
- [ ] [parallel] Add positive local-only and peer repair plus negative unnecessary peer repair, missed available item, incomplete observation, committed truncation, conflicting history, unknown quorum, term loss, vote loss, and bad snapshot tests. r[molten.consensus.storage_fault_recovery_progress] r[molten.consensus.storage_fault_validation]

## Evidence and closeout

- [ ] [serial] Add storage-fault caveats and cohort-bound receipt fields without widening Valence or Cairn claims. r[molten.consensus.storage_fault_claim_boundary]
- [ ] [parallel] Run pinned ChaosControl campaigns for flush error, write error, corruption, cache retention, restart, local-only recovery, peer repair, global absence, and incomplete observation. r[molten.consensus.storage_fault_recovery_progress] r[molten.consensus.storage_fault_validation] r[molten.fabric_durability.storage_fault_validation]
- [ ] [serial] Run focused tests, formatting, Clippy, Octet, Cairn gates, and relevant Nix checks. r[molten.consensus.storage_fault_validation] r[molten.fabric_durability.storage_fault_validation]

## Verification Coverage

- `Scenario: Synchronized commit is acknowledged` -> persistence shell and positive classification tests
- `Scenario: Commit result is uncertain` -> poison and reopen tests
- `Scenario: Snapshot publication becomes uncertain` -> parent-sync and startup reconciliation tests
- `Scenario: Local repair completes` -> voting-admission rejection test
- `Scenario: Quorum establishes an uncommitted entry` -> protocol repair test
- `Scenario: Commitment remains unknown` -> wait decision test
- `Scenario: Campaign cohort changes` -> evidence identity rejection test
