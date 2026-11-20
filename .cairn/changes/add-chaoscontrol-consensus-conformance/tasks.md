## Phase 1: Molten semantic projection

- [x] [serial] Define pure Molten chain observations over initial state, committed control-plane transitions, stable application refs, and lossless observer accounting. r[molten.consensus.chaoscontrol_chain_observation]
- [x] [serial] Map client-session outcomes to acknowledged, definitely rejected, or indefinite while preserving one logical operation identity across retries. r[molten.consensus.chaoscontrol_operation_identity]
- [x] [serial] Add tests for deterministic projection, changed order, duplicate apply, rollback, lossless gaps, malformed refs, stale generation, and indefinite outcomes. r[molten.consensus.chaoscontrol_chain_observation] r[molten.consensus.chaoscontrol_operation_identity]

## Phase 2: Producer contract and guest packaging

- [x] [depends:chaoscontrol-smr-chain-contract] [depends:chaoscontrol-protocol-observation-cohorts] Pin the immutable ChaosControl package revision, workload schema, observation envelope and cohort schemas, hash profile, and accepted evidence classes through Nix-owned inputs. r[molten.testing.chaoscontrol_smr_profile]
- [ ] [depends:chaoscontrol-smr-chain-contract] [depends:chaoscontrol-protocol-observation-cohorts] Implement a Rust consumer adapter that invokes the admitted Molten consensus and application paths. r[molten.consensus.chaoscontrol_chain_observation]
- [ ] [depends:molten-cross-process-consensus-guest] Package a bounded multi-node Molten guest cohort without workspace-relative runtime inputs. r[molten.testing.chaoscontrol_smr_profile]
- [x] [parallel] Add a deliberately faulty local adapter fixture for divergence, duplicate application, rollback, and stalled recovery diagnostics. r[molten.testing.chaoscontrol_smr_validation]

## Phase 3: Fault and recovery campaigns

- [x] [depends:chaoscontrol-smr-chain-contract] Add the no-fault control and bounded loss, reorder, partition, quorum-loss, leader-crash, follower-crash, restart, and snapshot catch-up profiles. r[molten.consensus.chaoscontrol_fault_profile]
- [x] [serial] Require continuous chain equality, apply-once operation identity, monotonic commit, and deterministic recovered state refs. r[molten.consensus.chaoscontrol_safety]
- [x] [serial] Require stable quorum, inactive disruptive faults, admitted lifecycle state, and a named virtual progress horizon before liveness evaluation. r[molten.consensus.chaoscontrol_liveness]
- [ ] [parallel] Share canonical operation corpora and invariant identifiers with whole-system simulation without sharing evidence-profile labels. r[molten.testing.chaoscontrol_smr_claim_boundary]

## Phase 4: Evidence import and operator workflow

- [x] [serial] Implement a pure fail-closed importer for producer, schema, Molten artifact, observer completeness, profile, bound, fault, observation, verdict, replay, and non-claim facts. r[molten.testing.chaoscontrol_smr_evidence]
- [x] [serial] Add a thin shell that imports a ChaosControl bundle and emits a canonical Molten external-evidence receipt. r[molten.testing.chaoscontrol_smr_evidence]
- [x] [parallel] Add operator status for cohort identities, campaign bounds, fault outcomes, safety, liveness preconditions, replay class, blockers, and non-claims. r[molten.testing.chaoscontrol_smr_profile]
- [x] [serial] Deny evidence that promotes KVM workload results into authority, security, Byzantine tolerance, production SLO, deployment, or release claims. r[molten.testing.chaoscontrol_smr_claim_boundary]

## Phase 5: Validation

- [ ] [serial] Run pure projection and importer tests, positive and negative fixtures, guest packaging, no-fault and fault campaigns, replay, claim-boundary, and offline bundle checks. r[molten.testing.chaoscontrol_smr_validation]
- [ ] [serial] Run Cairn validation and proposal, design, and tasks gates before sync or archive. r[molten.testing.chaoscontrol_smr_validation]

## Offline consumer cutover (2026-11-17)

- [x] [serial] Add an explicitly instrumentation-only pure importer with independently supplied operation/state policy, actual producer source/cohort/history/receipt validation, retained original/repeat semantic data and typed denials. This is completed local work beneath Phase 4; the full external execution importer task remains open. r[molten.testing.chaoscontrol_smr_evidence]
- [x] [parallel] Reconcile the inline projection domain and retained Molten provenance without repairing legacy hash-only SDK events. r[molten.consensus.chaoscontrol_chain_observation]
- [x] [serial] Exercise 47 selected ChaosControl tests, three bounded ledger read regressions and the real import/status CLI, including no-ledger-mutation deny paths. These are source/instrumentation controls, not guest, physical-fault or saved-snapshot observations. r[molten.testing.chaoscontrol_smr_validation]
- [x] [serial] Retain the clean workspace/library Octet producers, complete source corpus, passing strict source gate, remediation plan, selected hermetic Nix checks and byte-identical canonical artifacts in the [source/instrumentation checkpoint](evidence/consumer-cutover-20261117/qualification.json). Preserve the rejected directory-only corpus imports; this does not complete the unchecked execution or replay obligations. r[molten.testing.chaoscontrol_smr_validation]

The optional `chaoscontrol-conformance` feature selects `chaoscontrol-smr` at
`ec8aa799bad4735bf1028a092ca39feb08fb4bb7`; the snapshot-descriptor pin is unchanged.
Source data import and retained-policy status do not complete the admitted live
Raft/application adapter, measured guest package, actual fault/recovery campaigns
or execution/snapshot-attestation criteria. Those original obligations remain
unchecked; the change stays blocked and must not sync or archive as complete.

## Local fault and safety slice (2026-11-20)

[`profiles.rs`](../../../src/fabric_consistency/chaoscontrol/conformance/profiles.rs)
declares the no-fault control and finite loss, reorder, partition, quorum-loss,
leader/follower crash, restart, and snapshot catch-up campaigns; disk, clock,
and Byzantine requests are typed denials.
[`safety.rs`](../../../src/fabric_consistency/chaoscontrol/conformance/safety.rs)
binds operation identity and application receipts to the producer chain and is
now used by the fail-closed importer.
[`faulttests.rs`](../../../src/fabric_consistency/chaoscontrol/conformance/faulttests.rs)
exercises divergent, duplicate, rollback, and stalled adapters. The focused
selection passed 43 conformance tests; the [retained record](evidence/2026-11-20-fault-matrix-safety.md)
keeps the exact command and non-claims. The live adapter, measured guest
cohort, external campaigns, shared simulation corpus, and validation gates
remain open.

## Blocker

The SMR workload and protocol-observation contracts are archived in ChaosControl as `2026-08-07-add-smr-chain-workload` and `2026-11-05-add-protocol-observation-cohorts`. The consumer now pins `ec8aa799bad4735bf1028a092ca39feb08fb4bb7` and supplies the compatible inline encoder and bounded instrumentation importer. Missing producer specifications, an immutable source pin and the data-envelope handoff are no longer blockers.

Actual campaigns and imported execution evidence still require a measured cross-process Molten guest cohort with admitted distinct-guest transport and persistent-device mappings, complete source/build/runtime closure and actual committed-apply/independent-oracle observations. The current memory-backed supervisor and native three-child signal directory do not supply that contract. Neither an in-process model, native stdout, historical release receipt nor strict-SMP rejection can substitute for a supported single-vCPU-per-guest multi-VM observation.
