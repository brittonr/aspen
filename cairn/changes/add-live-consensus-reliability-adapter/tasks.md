## Phase 1: Workload and semantic core

- [ ] [serial] Define the typed service-registry register profile with exact key, initial value, value corpus, generator, seed, weights, clients, concurrency, operations, consistency model, recovery, and evidence bounds. r[molten.consensus.live_register_profile]
- [ ] [serial] Implement the pure bounded read/write generator with deterministic operation plans and complete choice records. r[molten.consensus.live_operation_generator]
- [ ] [serial] Implement pure public-operation projection and `ok`, `fail`, `info`, pending, retry, logical-ID, and attempt-ID mapping. r[molten.consensus.live_operation_history]
- [ ] [parallel] Add positive write/read fixtures and negative changed-ID, malformed-response, false-definite-failure, unsupported-value, and incomplete-pair fixtures. r[molten.testing.live_reliability_validation]

## Phase 2: Public client and deployment adapter

- [ ] [depends:chaoscontrol-semantic-history-contract] [serial] Pin the semantic-history v2 schema, register model, checker report, and evidence classes through immutable Nix inputs. r[molten.testing.live_reliability_cohort]
- [ ] [depends:onixos-live-reliability-contract] [serial] Implement the Molten product adapter for setup, write, read, recover, final-read, and teardown through public coordination endpoints. r[molten.consensus.live_public_adapter]
- [ ] [depends:onixos-molten-native-service] [serial] Package the exact multi-node Molten service cohort for disposable OnixOS clusters. r[molten.testing.live_reliability_cohort]
- [ ] [parallel] Add a deliberately faulty public fixture for stale reads, lost acknowledged writes, divergence, duplicate application, and stalled recovery. r[molten.testing.live_reliability_validation]

## Phase 3: Live fault and recovery profiles

- [ ] [depends:onixos-live-reliability-contract] [serial] Add no-fault, process-restart, temporary-partition, heal, recovery, and final-read profiles. r[molten.consensus.live_fault_profile]
- [ ] [serial] Require final public reads from every admitted endpoint after declared readiness and stable membership facts. r[molten.consensus.live_recovery]
- [ ] [parallel] Keep clock, durability, Byzantine, queue, lock, and transaction profiles explicitly unsupported. r[molten.consensus.live_fault_profile] r[molten.testing.live_reliability_claim_boundary]

## Phase 4: Checker and evidence integration

- [ ] [serial] Invoke the pinned register checker over complete live histories and retain valid, invalid, and unknown reports. r[molten.consensus.live_linearizability]
- [ ] [parallel] Add optional pinned reference-checker runs and block promotion on disagreement. r[molten.consensus.live_linearizability]
- [ ] [serial] Implement a pure fail-closed importer for producer, cohort, profile, generator, history, fault, recovery, checker, witness, teardown, and non-claim facts. r[molten.testing.live_reliability_evidence]
- [ ] [serial] Add a thin shell that stores a canonical external live-reliability receipt without granting other evidence or authority roles. r[molten.testing.live_reliability_evidence]
- [ ] [parallel] Compare simulation, NixOS VM, ChaosControl KVM, and live profiles without merging their evidence classes. r[molten.testing.live_reliability_claim_boundary]

## Phase 5: Validation and closeout

- [ ] [parallel] Add malformed history, incomplete recovery, unobserved fault, stale artifact, checker disagreement, teardown gap, and overclaim rejection fixtures. r[molten.testing.live_reliability_validation]
- [ ] [serial] Run pure adapter and importer tests, public client fixtures, disposable live campaigns, checker conformance, evidence validation, and Nix checks. r[molten.testing.live_reliability_validation]
- [ ] [serial] Run Cairn proposal, design, tasks, and validation gates before sync or archive. r[molten.testing.live_reliability_validation]

## Blocker

Live behavior tasks remain blocked until ChaosControl and OnixOS archive their semantic-history and live-reliability contracts. The production-shaped coordination endpoint and native OnixOS service profile must also be available.
