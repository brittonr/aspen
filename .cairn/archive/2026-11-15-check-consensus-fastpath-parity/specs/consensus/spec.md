# Consensus fast-path model evidence

## MODIFIED Requirements

### Requirement: The fault corpus checks fast-path composition invariants
r[molten.consensus.fast_path_model.fault_corpus] Molten MUST provide bounded positive and negative schedules for three-replica and five-replica profiles covering non-conflicting fast commit, conflict fallback, original-only operation, view-straddled acknowledgements, missing proposer promises, leader failure after fast reply, stale conflicting entries, partitions, quorum loss, interrupted and cascading recovery, restart, convergence, and duplicate suppression. The model MUST check recoverability, no conflicting predecessor, committed-order agreement, execution agreement, linearizable conflicting-command order, and at-most-once application.

The authoritative TLA+ source MUST map all six consensus profile invariant names to executable named properties, explicitly expose abstraction assumptions, and preserve finite-model nonclaims. Admission evidence MUST describe the fast-commit boundary, not later acknowledgement state.

#### Scenario: Missing promise
- **GIVEN** the same admitted finite source and configuration are selected for both semantic checkers with the proposer-promise guard enabled
- **WHEN** the campaign disables only the proposer-promise guard
- **THEN** both semantic checkers MUST report NoMissingPromise with a causal nonempty counterexample trace.

#### Scenario: Stale conflicting predecessor is detected
- GIVEN a new proposer carries an uncommitted conflicting command from an older view ahead of a recovered fast-committed command
- WHEN invariant evaluation examines the candidate execution order
- THEN the run fails with a no-conflicting-predecessor counterexample.

#### Scenario: Three-replica failure preserves only the original path
- GIVEN a three-replica profile loses one replica
- WHEN the remaining majority can run the base protocol but the fast superquorum requires every replica
- THEN the model reports fast-path unavailable and original-path availability separately
- AND does not promote fallback latency to fast-path success.

### Requirement: Model evidence is replayable and bounded
r[molten.consensus.fast_path_model.evidence] Molten MUST emit canonical model profile, source cohort, run, transition trace, fault, recovery, invariant, coverage, first-divergence, minimized-counterexample, and final-state evidence under explicit finite bounds. Exported repro bundles MUST identify the model/runtime inputs needed for deterministic replay and MUST NOT contain live-engine or measured-performance claims.

The pinned TLC and unchanged TLA-ts publication MUST execute the same finite source and configuration. Successful exhaustive runs MUST agree on verdict and state counts. Parse-only success, timeout, exception, missing checker, incomplete search and unsupported operators MUST fail closed. The campaign MUST retain raw output, timing, counts, configuration and counterexample evidence without declaring unexecuted campaigns successful.

#### Scenario: State-count divergence
- **GIVEN** pinned TLC and the unchanged TLA-ts publication have executed the same finite source and configuration to exhaustive completion
- **WHEN** successful exhaustive checker runs disagree on distinct or generated state counts
- **THEN** the parity rail MUST fail and retain both observations for investigation.

#### Scenario: Counterexample replays from canonical inputs
- GIVEN bounded exploration finds a recovery-order violation and minimizes its causal schedule
- WHEN the repro bundle is replayed with the same canonical model inputs
- THEN it reaches the same failure class and first violating boundary.

#### Scenario: Unexplored state space remains visible
- GIVEN configured bounds stop exploration before all eligible alternatives are visited
- WHEN evidence is finalized
- THEN coverage reports the unexplored alternatives
- AND the run cannot claim exhaustive verification.

### Requirement: Fast-path model claims remain bounded
r[molten.consensus.fast_path_model.nonclaims] Fast-path model evidence MUST state that it does not prove the external artifact, a live Molten base engine, real transport, durability, timing, production linearizability, throughput, latency improvement, Byzantine tolerance, interactive transactions, arbitrary conflict predicates, or release readiness. A stronger profile MUST require its own implementation and environment evidence.

Quint-to-Rust traces and optional Veil checks MUST NOT substitute for TLA+/TLC/TLA-ts acceptance. Missing toolchain/dependency integration or unexecuted evidence MUST remain explicit, and model agreement MUST NOT grant runtime or release authority.

#### Scenario: Benchmark citation cannot admit production
- GIVEN the source cohort reports lower latency in an external geo-distributed benchmark
- WHEN Molten production admission evaluates only model and citation evidence
- THEN admission denies
- AND identifies missing live implementation, environment, failure, and performance evidence.

### Requirement: Fast-path model validation covers success and failure
r[molten.consensus.fast_path_model.validation] Molten MUST include positive and negative tests for profile admission, quorum derivation, conflict classification, stable-view promises, fallback identity, duplicate suppression, view-change recovery, recovery ordering, fault schedules, invariants, deterministic replay, minimization, source-reference conformance, bounded evidence, non-claims, and live/production denial.

The dev-only Quint Driver MUST call transition_recovery for all four recovery variants, evaluate_stable_view for fast admission, and ApplicationLedger::converge for convergence. State MUST compare phase, views, recovery sets and applied commands after every step, including rejected actions. Missing or drifting Quint MUST fail, not skip. A retained deterministic causal fixture MUST cause divergence when the real Rust mixed-view guard is removed in an isolated source copy. No runtime dependency or production guard weakening is permitted.

#### Scenario: Mutated mixed-view guard
- **GIVEN** a retained deterministic causal fixture exercises mixed-view acknowledgement rejection through the real Rust admission transition
- **WHEN** the isolated Rust mutation removes mixed-view acknowledgement rejection
- **THEN** the same deterministic replay MUST reject the divergent admission result and retain its generated trace and replay log.

#### Scenario: Valid bounded model suite passes
- GIVEN admitted three-replica and five-replica profiles and their complete positive and negative fixture cohorts
- WHEN focused validation runs
- THEN expected safe traces pass, expected hazards produce the named counterexamples, and model-only evidence validates offline.

#### Scenario: False non-conflict fixture fails
- GIVEN a fixture deliberately classifies two response-dependent commands as non-conflicting
- WHEN exploration finds an execution that changes state or response order
- THEN the semantic invariant fails
- AND the fixture cannot satisfy conflict-contract or production evidence gates.
