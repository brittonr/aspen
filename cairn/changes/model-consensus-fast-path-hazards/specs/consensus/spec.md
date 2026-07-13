## ADDED Requirements

### Requirement: Fast-path hazard models are explicit and model-only
r[molten.consensus.fast_path_model.profile] Molten MUST represent a consensus fast-path hazard model with an explicit profile identity, source-reference cohort, base-engine model identity, crash-fault assumptions, node and proposer bounds, derived majority and superquorum rules, command/conflict profile, view and recovery bounds, invariant set, evidence profile, and non-claims. The profile MUST remain pure-model or deterministic-simulation only and MUST deny live or production engine selection.

#### Scenario: Complete three-replica model is admitted
- GIVEN a bounded three-replica crash-fault profile with a pinned source cohort, compatible base model, derived quorum rules, conflict contract, invariants, and model-only claim profile
- WHEN model preflight runs
- THEN it produces a canonical admitted model plan
- AND reports that the fast-path superquorum contains every replica.

#### Scenario: Model profile cannot select production
- GIVEN a structurally valid fast-path model has no distinct-process live evidence
- WHEN a live or production group attempts to select it
- THEN selection denies before runtime construction
- AND diagnostics preserve the pure-model claim boundary.

### Requirement: Base models declare fast-path ordering prerequisites
r[molten.consensus.fast_path_model.base_prerequisites] A fast-path model MUST bind evidence that the base model preserves proposal order in log and execution order for conflicting commands proposed by one proposer and preserves proposer receive order in proposal order. A model whose buffering can reorder receive and proposal MAY remain compatible only when fast acknowledgement waits for equivalent proposal-order evidence. A model that can reorder conflicting proposals at execution MUST deny transparent fast-path compatibility.

#### Scenario: Ordered proposer model is compatible
- GIVEN a base model appends conflicting commands in proposal order and executes them in log order
- WHEN fast-path compatibility validates its declared proposer contract
- THEN the ordering prerequisite may pass subject to the remaining quorum, conflict, view, and recovery requirements.

#### Scenario: Buffered reorder requires a later acknowledgement boundary
- GIVEN a base proposer may receive command A before command B but buffer and propose B first
- WHEN the acceleration layer can observe only receive order
- THEN transparent receive-time acknowledgement denies
- AND compatibility requires proposal-order evidence or original-path fallback.

#### Scenario: Execution reorder is incompatible
- GIVEN a base model may propose conflicting command A before command B but execute B first
- WHEN compatibility admission runs
- THEN the transparent fast-path profile denies
- AND does not treat model-checking of the base protocol alone as sufficient.

### Requirement: Conflict classification is pure, versioned, and conservative
r[molten.consensus.fast_path_model.conflict_contract] A modeled fast path MUST bind a versioned extension-owned conflict contract to exact command and state-machine schemas. The conflict function MUST be deterministic and side-effect free and MUST report conflict whenever command order can affect application state or either command response. Unknown schemas, aliases, ranges, predicates, preconditions, analysis failures, and unsupported operations MUST conservatively conflict and use the original path.

#### Scenario: Independent keys can use the fast path
- GIVEN two key-value commands address distinct canonical keys and their responses do not depend on shared state
- WHEN the bound conflict contract evaluates them
- THEN it may classify them as non-conflicting for fast-path modeling.

#### Scenario: Unknown dependency falls back safely
- GIVEN a command contains an unsupported range predicate or unresolved alias
- WHEN conflict classification cannot establish independence
- THEN it classifies the command as conflicting
- AND the command remains eligible for the original path rather than being rejected or fast-committed.

### Requirement: Stable-view fast commit requires one view and all proposer promises
r[molten.consensus.fast_path_model.stable_view] A modeled fast commit MUST bind one acceleration view and one matching base-engine view, obtain acknowledgements from the derived same-view fast superquorum, and include a compatible ordering promise from every active original-path proposer in that view. Acknowledgements or promises from different views MUST NOT combine into a fast commit.

#### Scenario: Same-view superquorum commits
- GIVEN a command is conflict-free, both paths are in the same normal view, the fast superquorum acknowledges that view, and every active proposer promises compatible ordering
- WHEN the client evaluates the attempt
- THEN the model may classify the command as fast-committed.

#### Scenario: View-straddled acknowledgements fail
- GIVEN individually valid acknowledgements were issued across two acceleration or base-engine views
- WHEN their union would meet the numeric superquorum size
- THEN the fast commit still fails
- AND the original path remains available for fallback.

### Requirement: Both paths converge on one canonical operation
r[molten.consensus.fast_path_model.fallback_identity] The modeled fast and original paths MUST carry the same canonical command ref, client session and sequence, group, extension generation, application schema, policy/authority/resource cohort, and engine epoch. Fast-path failure MUST fall back to the original path without changing operation identity. Convergence MUST apply and reply to the operation at most once.

#### Scenario: Conflict falls back without changing identity
- GIVEN a fast attempt encounters an in-flight conflicting command
- WHEN the fast superquorum cannot form
- THEN the original path continues with the same canonical operation identity
- AND a later commit applies the operation once.

#### Scenario: Duplicate path completion does not duplicate effects
- GIVEN the client observed a fast commit and the original path later reaches the same command
- WHEN the state machine processes the converged record
- THEN client-session and command identity suppress duplicate application and duplicate authoritative reply.

### Requirement: View changes recover and order prior fast commits first
r[molten.consensus.fast_path_model.view_change_recovery] The modeled acceleration layer MUST track a view independently from the base engine. After a base-engine view change, it MUST pause new fast admission, agree on the last normal view's recoverable fast-command set, carry any previously accepted recovery set forward, commit the recovery set or an explicit no-op recovery marker through the original path, and only then admit commands in the new normal view. Recovered commands MUST precede every conflicting uncommitted command admitted by the new view.

#### Scenario: Leader fails after fast reply
- GIVEN a client received a valid fast reply and the original-path proposer fails before canonical commit
- WHEN a new proposer recovers the last normal view
- THEN the acknowledged command appears in the agreed recovery set
- AND commits through the original path before conflicting new-view commands.

#### Scenario: Empty recovery still creates a boundary
- GIVEN recovery finds no possibly fast-committed command in the last normal view
- WHEN the new proposer completes recovery
- THEN it commits an explicit no-op recovery marker before accepting normal new-view work.

### Requirement: The fault corpus checks fast-path composition invariants
r[molten.consensus.fast_path_model.fault_corpus] Molten MUST provide bounded positive and negative schedules for three-replica and five-replica profiles covering non-conflicting fast commit, conflict fallback, original-only operation, view-straddled acknowledgements, missing proposer promises, leader failure after fast reply, stale conflicting entries, partitions, quorum loss, interrupted and cascading recovery, restart, convergence, and duplicate suppression. The model MUST check recoverability, no conflicting predecessor, committed-order agreement, execution agreement, linearizable conflicting-command order, and at-most-once application.

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

#### Scenario: Counterexample replays from canonical inputs
- GIVEN bounded exploration finds a recovery-order violation and minimizes its causal schedule
- WHEN the repro bundle is replayed with the same canonical model inputs
- THEN it reaches the same failure class and first violating boundary.

#### Scenario: Unexplored state space remains visible
- GIVEN configured bounds stop exploration before all eligible alternatives are visited
- WHEN evidence is finalized
- THEN coverage reports the unexplored alternatives
- AND the run cannot claim exhaustive verification.

### Requirement: External reference conformance does not transfer proof
r[molten.consensus.fast_path_model.reference_conformance] Molten SHOULD compare independently expressed named scenarios, assumptions, and invariant outcomes against the pinned Jetpack paper and artifact cohort. Reference conformance MUST record source identity, compared behavior, mismatches, unsupported assumptions, and license posture, and MUST NOT treat external TLA+ success, tests, or benchmarks as proof of Molten code or performance.

#### Scenario: Reference mismatch blocks conformance
- GIVEN a Molten recovery scenario permits new-view work before the recovery marker while the pinned reference requires recovery priority
- WHEN reference conformance runs
- THEN it reports the semantic mismatch
- AND does not issue a passing conformance decision.

### Requirement: Fast-path model claims remain bounded
r[molten.consensus.fast_path_model.nonclaims] Fast-path model evidence MUST state that it does not prove the external artifact, a live Molten base engine, real transport, durability, timing, production linearizability, throughput, latency improvement, Byzantine tolerance, interactive transactions, arbitrary conflict predicates, or release readiness. A stronger profile MUST require its own implementation and environment evidence.

#### Scenario: Benchmark citation cannot admit production
- GIVEN the source cohort reports lower latency in an external geo-distributed benchmark
- WHEN Molten production admission evaluates only model and citation evidence
- THEN admission denies
- AND identifies missing live implementation, environment, failure, and performance evidence.

### Requirement: Fast-path model validation covers success and failure
r[molten.consensus.fast_path_model.validation] Molten MUST include positive and negative tests for profile admission, quorum derivation, conflict classification, stable-view promises, fallback identity, duplicate suppression, view-change recovery, recovery ordering, fault schedules, invariants, deterministic replay, minimization, source-reference conformance, bounded evidence, non-claims, and live/production denial.

#### Scenario: Valid bounded model suite passes
- GIVEN admitted three-replica and five-replica profiles and their complete positive and negative fixture cohorts
- WHEN focused validation runs
- THEN expected safe traces pass, expected hazards produce the named counterexamples, and model-only evidence validates offline.

#### Scenario: False non-conflict fixture fails
- GIVEN a fixture deliberately classifies two response-dependent commands as non-conflicting
- WHEN exploration finds an execution that changes state or response order
- THEN the semantic invariant fails
- AND the fixture cannot satisfy conflict-contract or production evidence gates.
