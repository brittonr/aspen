## Phase 1: Reference and model profile

- [ ] [serial] Define a typed Nickel model profile binding the pinned paper/artifact identity, crash-fault assumptions, named node/command/key/view/step bounds, derived quorum rules, base-engine prerequisites, conflict-contract ref, and model-only non-claims. r[molten.consensus.fast_path_model.profile]
- [ ] [depends:fast-path-model-profile] Implement pure profile validation, base receive/propose/execute ordering prerequisite admission, and quorum derivation for named three-replica and five-replica envelopes without hard-coded runtime thresholds. r[molten.consensus.fast_path_model.profile] r[molten.consensus.fast_path_model.base_prerequisites] r[molten.consensus.fast_path_model.stable_view]
- [ ] [parallel] Add positive complete profiles and negative unknown-reference, unsupported-fault-model, malformed-bound, impossible-quorum, live-selection, production-selection, and claim-overreach fixtures. r[molten.consensus.fast_path_model.profile] r[molten.consensus.fast_path_model.nonclaims]

## Phase 2: Stable-view composition

- [ ] [serial] Implement the pure dual-path transition model over one canonical command/session identity, including original-only operation, fast attempt, same-view superquorum, all-proposer promises, fallback, convergence, and at-most-once application. r[molten.consensus.fast_path_model.stable_view] r[molten.consensus.fast_path_model.fallback_identity]
- [ ] [parallel] Add the versioned extension-owned conflict contract, conservative unknown handling, schema binding, and semantic conflict-oracle fixtures covering keys, reads/writes, ranges, aliases, response dependencies, and preconditions. r[molten.consensus.fast_path_model.conflict_contract]
- [ ] [parallel] Add positive non-conflicting fast commits and safe fallback fixtures plus negative receive/propose reorder, proposal/execution reorder, mixed-view quorum, missing-proposer promise, identity mismatch, duplicate application, and false-non-conflict invariant fixtures. r[molten.consensus.fast_path_model.base_prerequisites] r[molten.consensus.fast_path_model.stable_view] r[molten.consensus.fast_path_model.conflict_contract] r[molten.consensus.fast_path_model.fallback_identity]

## Phase 3: View change and recovery

- [ ] [serial] Add independent acceleration-view transitions, base-view synchronization, recovery pause, prior-normal-view recovery-set agreement, accepted-set carry-forward, original-path recovery/no-op markers, and resume ordering. r[molten.consensus.fast_path_model.view_change_recovery]
- [ ] [parallel] Add leader-failure-after-fast-reply, view-straddled acknowledgement, stale-conflict-before-recovery, empty recovery, interrupted recovery, cascading view failure, partition, rejoin, and replica-restart schedules. r[molten.consensus.fast_path_model.view_change_recovery] r[molten.consensus.fast_path_model.fault_corpus]
- [ ] [parallel] Add invariants for recoverability, no conflicting predecessor, committed-log agreement, execution-order agreement, linearizable conflicting-command order, and duplicate suppression. r[molten.consensus.fast_path_model.fault_corpus]

## Phase 4: Evidence and external conformance

- [ ] [depends:fast-path-model-recovery] Add bounded exploration, deterministic replay, first-divergence diagnostics, causal counterexample minimization, and canonical profile/run/trace/invariant/coverage/recovery artifacts. r[molten.consensus.fast_path_model.fault_corpus] r[molten.consensus.fast_path_model.evidence]
- [ ] [parallel] Compare independently expressed scenario outcomes and invariant names against the pinned Jetpack paper and artifact TLA+ cohort, recording mismatches and unsupported assumptions without treating external model checks as Molten proof. r[molten.consensus.fast_path_model.reference_conformance] r[molten.consensus.fast_path_model.nonclaims]
- [ ] [depends:fast-path-model-evidence] Export minimal fault repro bundles suitable for later whole-system simulation and ChaosControl ingestion, with exact model-only claim labels and no live or performance promotion. r[molten.consensus.fast_path_model.evidence] r[molten.consensus.fast_path_model.nonclaims]

## Phase 5: Validation

- [ ] [depends:fast-path-model-fault-corpus] Run positive and negative profile, base-ordering prerequisite, conflict, stable-view, fallback, recovery, invariant, replay, minimization, reference-conformance, evidence, and production-denial tests. r[molten.consensus.fast_path_model.validation]
- [ ] [serial] [depends:fast-path-model-validation] Run focused formatting and tests, Cairn validation, proposal/design/tasks gates, traceability coverage, and the smallest relevant Nix checks before sync and archive. r[molten.consensus.fast_path_model.validation]
