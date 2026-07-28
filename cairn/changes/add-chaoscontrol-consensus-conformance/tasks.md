## Phase 1: Molten semantic projection

- [ ] [serial] Define pure Molten chain observations over initial state, committed control-plane transitions, stable application refs, and lossless observer accounting. r[molten.consensus.chaoscontrol_chain_observation]
- [ ] [serial] Map client-session outcomes to acknowledged, definitely rejected, or indefinite while preserving one logical operation identity across retries. r[molten.consensus.chaoscontrol_operation_identity]
- [ ] [serial] Add tests for deterministic projection, changed order, duplicate apply, rollback, lossless gaps, malformed refs, stale generation, and indefinite outcomes. r[molten.consensus.chaoscontrol_chain_observation] r[molten.consensus.chaoscontrol_operation_identity]

## Phase 2: Producer contract and guest packaging

- [ ] [depends:chaoscontrol-smr-chain-contract] Pin the immutable ChaosControl package revision, workload schema ref, hash profile ref, and accepted evidence classes through Nix-owned inputs. r[molten.testing.chaoscontrol_smr_profile]
- [ ] [depends:chaoscontrol-smr-chain-contract] Implement a Rust consumer adapter that invokes the admitted Molten consensus and application paths. r[molten.consensus.chaoscontrol_chain_observation]
- [ ] [depends:molten-cross-process-consensus-guest] Package a bounded multi-node Molten guest cohort without workspace-relative runtime inputs. r[molten.testing.chaoscontrol_smr_profile]
- [ ] [parallel] Add a deliberately faulty local adapter fixture for divergence, duplicate application, rollback, and stalled recovery diagnostics. r[molten.testing.chaoscontrol_smr_validation]

## Phase 3: Fault and recovery campaigns

- [ ] [depends:chaoscontrol-smr-chain-contract] Add the no-fault control and bounded loss, reorder, partition, quorum-loss, leader-crash, follower-crash, restart, and snapshot catch-up profiles. r[molten.consensus.chaoscontrol_fault_profile]
- [ ] [serial] Require continuous chain equality, apply-once operation identity, monotonic commit, and deterministic recovered state refs. r[molten.consensus.chaoscontrol_safety]
- [ ] [serial] Require stable quorum, inactive disruptive faults, admitted lifecycle state, and a named virtual progress horizon before liveness evaluation. r[molten.consensus.chaoscontrol_liveness]
- [ ] [parallel] Share canonical operation corpora and invariant identifiers with whole-system simulation without sharing evidence-profile labels. r[molten.testing.chaoscontrol_smr_claim_boundary]

## Phase 4: Evidence import and operator workflow

- [ ] [serial] Implement a pure fail-closed importer for producer, schema, Molten artifact, observer completeness, profile, bound, fault, observation, verdict, replay, and non-claim facts. r[molten.testing.chaoscontrol_smr_evidence]
- [ ] [serial] Add a thin shell that imports a ChaosControl bundle and emits a canonical Molten external-evidence receipt. r[molten.testing.chaoscontrol_smr_evidence]
- [ ] [parallel] Add operator status for cohort identities, campaign bounds, fault outcomes, safety, liveness preconditions, replay class, blockers, and non-claims. r[molten.testing.chaoscontrol_smr_profile]
- [ ] [serial] Deny evidence that promotes KVM workload results into authority, security, Byzantine tolerance, production SLO, deployment, or release claims. r[molten.testing.chaoscontrol_smr_claim_boundary]

## Phase 5: Validation

- [ ] [serial] Run pure projection and importer tests, positive and negative fixtures, guest packaging, no-fault and fault campaigns, replay, claim-boundary, and offline bundle checks. r[molten.testing.chaoscontrol_smr_validation]
- [ ] [serial] Run Cairn validation and proposal, design, and tasks gates before sync or archive. r[molten.testing.chaoscontrol_smr_validation]

## Blocker

The KVM producer and import tasks depend on an archived versioned ChaosControl
SMR chain-workload contract. Multi-node behavior tasks also require an admitted
cross-process Molten consensus guest. Missing either dependency must remain a
typed blocker and cannot be replaced by a local in-process consensus model.
