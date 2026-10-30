# Durable State Ports Delta

## ADDED Requirements

### Requirement: Explicit storage-fault outcomes
r[molten.fabric_durability.storage_fault_outcomes] Molten MUST distinguish requested durability from observed persistence and MUST represent an uncertain commit or synchronization result as `OutcomeUnknown`.

#### Scenario: Synchronized commit is acknowledged
- GIVEN an admitted durable mutation and a shell observation that satisfies the selected durability profile
- WHEN the pure core classifies the observation
- THEN it returns a durable acknowledgement bound to that observation

#### Scenario: Commit result is uncertain
- GIVEN an admitted mutation and a commit or synchronization error that cannot prove non-application
- WHEN the pure core classifies the observation
- THEN it returns `OutcomeUnknown` and does not return durable success

### Requirement: Adapter quarantine after storage faults
r[molten.fabric_durability.adapter_quarantine] The storage shell MUST quarantine an adapter after uncertain, corrupt, repaired, or inconsistent persistent-state observations until explicit reopen and reconciliation succeeds.

#### Scenario: Quarantined adapter receives a request
- GIVEN a quarantined adapter
- WHEN a caller requests a normal read or mutation
- THEN the shell rejects the request without using cached state as durable truth

#### Scenario: Failed synchronization is retried
- GIVEN a synchronization call returned an uncertain error
- WHEN the same call later returns success without reopen and reconciliation
- THEN the shell keeps the adapter quarantined

### Requirement: Durable snapshot publication
r[molten.fabric_durability.snapshot_publication] Molten MUST publish snapshot payloads through a pinned capability-relative durable file publication contract before it commits visible snapshot metadata.

#### Scenario: Snapshot publication succeeds
- GIVEN a staged snapshot whose BLAKE3 identity matches its request
- WHEN payload and parent synchronization complete and metadata commits
- THEN the snapshot becomes visible with the matching content identity

#### Scenario: Parent synchronization fails
- GIVEN a snapshot destination rename completed
- WHEN parent synchronization fails
- THEN Molten records committed durability unknown and reconciles the destination before metadata repair

### Requirement: Storage-fault validation
r[molten.fabric_durability.storage_fault_validation] Molten MUST test successful durability and negative commit, synchronization, corruption, cache, reopen, and publication outcomes.

#### Scenario: Negative storage fixture runs
- GIVEN a selected fault fixture
- WHEN the focused validation suite runs
- THEN the suite checks that no durable acknowledgement or normal operation bypasses quarantine
