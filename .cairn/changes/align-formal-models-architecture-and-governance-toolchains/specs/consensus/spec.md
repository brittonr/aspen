# Consensus deltas

## ADDED Requirements

### Requirement: Production Raft has non-tautological CI model checking
r[molten.consensus.production_raft_formal_model] Production replicated Raft (`fabric_consistency::raft`) MUST be backed by a CI-executed formal model covering leader election, log replication, conflicting-suffix truncation, compaction, and `ReadIndex` confirmation without assuming Leader Completeness in `Init`. Fast-path models MUST genuinely model `Quorum` intersection and per-replica state rather than centralized guard tautologies.

#### Scenario: Missing election or truncation safety violates model invariant
- GIVEN a production Raft model mutation that permits committing a previous-term entry directly or truncating committed entries
- WHEN TLC or Quint checks the model
- THEN the model checker MUST emit a counterexample trace and fail CI.

#### Scenario: Fast-path model exercises quorum intersection
- GIVEN competing superquorums and message reordering in `ConsensusFastPath.tla`
- WHEN the model runs with an insufficient quorum size
- THEN the safety invariant MUST fail rather than passing vacuously with an unused `Quorum` constant.

## MODIFIED Requirements

### Requirement: The fault corpus checks fast-path composition invariants
r[molten.consensus.fast_path_model.fault_corpus] Molten MUST provide bounded positive and negative schedules for three-replica and five-replica profiles covering non-conflicting fast commit, conflict fallback, original-only operation, view-straddled acknowledgements, missing proposer promises, leader failure after fast reply, stale conflicting entries, partitions, quorum loss, interrupted and cascading recovery, restart, convergence, and duplicate suppression. The model MUST check recoverability, no conflicting predecessor, committed-order agreement, execution agreement, linearizable conflicting-command order, and at-most-once application.

The authoritative TLA+ source MUST map all six consensus profile invariant names to executable named properties, explicitly expose abstraction assumptions, and preserve finite-model nonclaims. Admission evidence MUST describe the fast-commit boundary, not later acknowledgement state. Properties that are derived projections of another checked property MUST be identified as derived rather than presented as independent evidence. Baseline configurations MUST use the derived majority and superquorum unless they are explicitly marked as a non-derived quorum-intersection exercise; such an exercise MUST state that its quorum sizes differ from the derived rules and why. A model condition the Rust API does not enforce MUST be documented as a model assumption and tracked as a source discrepancy.

#### Scenario: Missing promise
- **GIVEN** the same admitted finite source and configuration are selected for all three semantic checkers with the proposer-promise guard enabled
- **WHEN** the campaign disables only the proposer-promise guard
- **THEN** every semantic checker MUST report `CommittedOrderAgreement` with a causal counterexample trace of more than one state, in which the original engine decides a different command at a fast-committed position.

#### Scenario: Insufficient quorum breaks unique recovery selection
- **GIVEN** a configuration in which `QuorumSize + 2 * SuperQuorumSize <= 2 * N`, so the commands held by `QuorumSize + SuperQuorumSize - N` recovery reports are not unique (three replicas with a two-replica superquorum, or the non-derived four-replica exercise with a two-replica recovery quorum)
- **WHEN** all three semantic checkers run it
- **THEN** each MUST report `CommittedOrderAgreement` with a causal counterexample trace through view-change recovery.

#### Scenario: Stale conflicting predecessor is detected
- GIVEN a new proposer carries an uncommitted conflicting command from an older view ahead of a recovered fast-committed command
- WHEN invariant evaluation examines the candidate execution order
- THEN the run fails with a no-conflicting-predecessor counterexample.

#### Scenario: Three-replica failure preserves only the original path
- GIVEN a three-replica profile loses one replica
- WHEN the remaining majority can run the base protocol but the fast superquorum requires every replica
- THEN the model reports fast-path unavailable and original-path availability separately
- AND does not promote fallback latency to fast-path success.

### Requirement: Dive independently corroborates consensus fast-path model parity
r[molten.consensus.fast_path_model.dive_parity] The x86_64 Linux consensus fast-path model parity check MUST run pinned Dive 0.1.0 in addition to the existing TLC and TLA-ts runs, using the same unchanged model and configurations for every configured baseline and mutant. TLC MUST remain authoritative. The campaign MUST preserve the 180 second checker/config and 720 second total limits across all of its sequential checker runs.

#### Scenario: Three exhaustive baseline results agree
- GIVEN each configured finite baseline with its temporal property: three replicas with the derived quorums, and an explicitly non-derived four-replica quorum-intersection exercise (superquorum 3 and recovery quorum 3, where the derived four-replica superquorum is 4)
- WHEN all three pinned checkers run within the agreed limits
- THEN all MUST report success with an empty queue and the same generated and distinct counts
- AND Dive MUST report a completed, exhausted temporal check.

#### Scenario: Three bounded mutant results agree
- GIVEN each configured mutant (missing promise, three-replica insufficient superquorum, four-replica small recovery quorum)
- WHEN the checkers exercise the mutant
- THEN each MUST report `CommittedOrderAgreement` with a causal trace of more than one state
- AND early-stop counts MUST remain observations, not parity assertions.

#### Scenario: Unqualified observations fail closed
- GIVEN archive or version drift, an unknown schema, a refusal, a stop, a malformed or incomplete report, a timeout, or a parity mismatch
- WHEN the campaign evaluates the checker observations
- THEN it MUST fail with the named cause and MUST NOT claim parity.

#### Scenario: Evidence remains finite-model-only
- GIVEN retained campaign evidence
- WHEN a reader inspects the result
- THEN it MUST identify the Dive pin, options and durations and bind the raw checker outputs and reports with BLAKE3
- AND agreement with TLC MUST NOT be presented as proof of Aspen's Rust implementation.
