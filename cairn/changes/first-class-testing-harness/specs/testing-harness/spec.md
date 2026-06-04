## ADDED Requirements

### Requirement: First-class harness artifacts
r[molten.testing.harness_artifacts] The system MUST represent test suites, cases, steps, fixtures, oracles, runs, and reports as canonical artifacts that identify their dependency closure, policy refs, schema refs, handler profile, seed or effect-log refs, runner version, and initial/final state hashes.

#### Scenario: Deterministic suite identity is complete
r[molten.testing.harness_artifacts.identity]
- GIVEN a deterministic test suite with fixtures and expected outcomes
- WHEN the harness computes the suite and run identity
- THEN the identity includes the suite artifact, dependency closure, initial state, schema refs, policy refs, handler profile config, seed or effect-log ref, runner version, and relevant runtime/tool versions

### Requirement: Determinism and replay are core harness invariants
r[molten.testing.determinism_replay_core] The harness MUST treat deterministic identity, record/replay support, and first-divergence replay diagnostics as core requirements for integration, transcript, property, chaos, dogfood, CI-evidence, and admission-evidence runs. A run that performs nondeterministic admitted external effects MUST record canonical effect logs sufficient to replay or MUST be marked non-replayable and ineligible as deterministic evidence.

#### Scenario: Evidence-bearing run declares replay status
r[molten.testing.determinism_replay_core.status]
- GIVEN a harness run intended for CI evidence or policy admission evidence
- WHEN the run report is finalized
- THEN the report identifies whether the run was deterministic, replayed, recorded for replay, or non-replayable and excludes non-replayable runs from deterministic evidence gates

#### Scenario: Recorded run can be replayed through the harness
r[molten.testing.determinism_replay_core.record_replay]
- GIVEN a record-mode harness run with admitted external adapter observations
- WHEN the harness replays the run using the recorded effect log and same runtime identity
- THEN replay injects recorded responses, denies live external effects, and compares canonical trace, receipt, output, and final state hashes

### Requirement: Preflight implementation guards
r[molten.testing.preflight_guards] Before the harness is accepted as evidence-bearing infrastructure, the system MUST define and enforce guards for harness privileges, deterministic hermeticity, schema/version discipline, fail-closed evidence, fixture mutation visibility, production/test separation, secret and capability hygiene, golden update governance, resource/logical-time bounds, scheduler/liveness outcomes, adapter contract gates, and replay eligibility gates.

#### Scenario: Harness bypass requires admitted capability
r[molten.testing.preflight_guards.privilege_boundary]
- GIVEN a test helper that needs to inspect or perturb runtime internals
- WHEN the helper is used by an evidence-bearing harness run
- THEN the helper operates through an explicit admitted test capability and emits canonical trace and receipt evidence rather than using an invisible private backdoor

#### Scenario: Deterministic run rejects ambient input
r[molten.testing.preflight_guards.hermeticity]
- GIVEN a deterministic harness mode
- WHEN a step attempts to read ambient filesystem, environment, network, wall-clock, entropy, process state, or OS scheduling state outside declared fixtures or effect logs
- THEN the run is denied or fails with a hermeticity diagnostic before the observation affects semantic runtime state

#### Scenario: Evidence fails closed
r[molten.testing.preflight_guards.fail_closed]
- GIVEN an evidence-bearing harness run
- WHEN a required trace, receipt, state hash, effect record, handler profile identity, schema version, or replay identity is missing
- THEN the run fails rather than treating absent evidence as success

#### Scenario: Production profile excludes test-only surfaces
r[molten.testing.preflight_guards.production_separation]
- GIVEN a production or release-admission profile
- WHEN a test-only fixture, bypass capability, debug hook, or exploratory non-replayable profile is requested
- THEN the runtime denies the request unless an explicit policy admits it for record, replay, or debug use and records evidence

#### Scenario: Golden output update is governed
r[molten.testing.preflight_guards.golden_governance]
- GIVEN a proposed change to a golden trace, receipt, state hash, snapshot, or report artifact
- WHEN the change is accepted
- THEN the update records old and new refs, review or policy authority, reason class, migration notes where applicable, and receipts

#### Scenario: Resource exhaustion is deterministic
r[molten.testing.preflight_guards.resource_bounds]
- GIVEN a harness run with declared bounds for turns, scheduler steps, logical time, effects, trace bytes, queues, assertions, storage/blob/network bytes, Wasm fuel, Steel/native checkpoints, or job-stage resources
- WHEN a bound is exceeded
- THEN the run fails with a deterministic resource diagnostic and canonical evidence

#### Scenario: Replay eligibility gates exclude exploratory runs
r[molten.testing.preflight_guards.replay_eligibility]
- GIVEN a CI, release, upgrade, admission, or policy evidence gate
- WHEN a harness run is exploratory or non-replayable
- THEN the run is excluded from satisfying the gate even if its rendered status is pass

### Requirement: Preserves communication rail
r[molten.testing.preserves_comm_rail] The testing harness MUST use canonical Preserves values or Molten envelopes for semantically relevant communication across the harness/runtime boundary, including control commands, actor stimuli, dataspace assertions/retractions, Observe patterns, adapter fixtures, effect requests/responses, observations, traces, receipts, diagnostics, oracles, and reports.

#### Scenario: Harness injects actor stimulus through Preserves
r[molten.testing.preserves_comm_rail.actor_stimulus]
- GIVEN a harness step that sends a message or publishes a dataspace assertion
- WHEN the step crosses into the runtime
- THEN the stimulus is represented as canonical Preserves data or a Molten envelope with a stable boundary hash before delivery

#### Scenario: Harness observes runtime outcome through Preserves
r[molten.testing.preserves_comm_rail.observation]
- GIVEN a runtime turn that commits actions and emits trace records
- WHEN the harness records the outcome
- THEN delivered envelopes, visible assertions, committed actions, trace records, receipt refs, and diagnostics are captured as canonical Preserves values or content refs

#### Scenario: Text reports are not primary evidence
r[molten.testing.preserves_comm_rail.rendering]
- GIVEN a harness run that emits terminal, markdown, JSON, JUnit, or TAP output
- WHEN an oracle or report identity is evaluated
- THEN the rendered text is treated as a view over canonical Preserves records rather than the primary evidence or matching oracle

### Requirement: Canonical harness hashes
r[molten.testing.boundary_hashes] The harness MUST compute test identity, oracle matching refs, replay-log refs, cache keys, trace refs, receipt refs, and report refs from Blake3 hashes over canonical Preserves bytes or authenticated content refs, not from Rust debug formatting, map iteration order, or terminal rendering.

#### Scenario: Equivalent expected values hash identically
r[molten.testing.boundary_hashes.equivalent_values]
- GIVEN two equivalent expected Preserves values constructed through different Rust or transcript code paths
- WHEN the harness canonicalizes and hashes them
- THEN both values produce the same oracle hash

### Requirement: Fresh deterministic local runner
r[molten.testing.fresh_local_runner] The harness MUST provide a fresh deterministic local runner that starts an isolated in-process runtime, installs declared artifacts, binds declared policy and handler profile inputs, executes steps through the Preserves rail, records canonical evidence, and cleans up fixture state by default.

#### Scenario: Fresh run does not depend on ambient state
r[molten.testing.fresh_local_runner.no_ambient_state]
- GIVEN two executions of the same deterministic suite on the same artifacts and seed
- WHEN the fresh local runner executes them
- THEN both runs start from the declared fixture state rather than ambient local state and produce the same canonical report refs

### Requirement: Fixture adapters preserve effect boundaries
r[molten.testing.fixture_adapters] Fixture, fake, mock, simulator, and chaos adapters used by the harness MUST communicate via canonical effect request/response records and MUST NOT mutate semantic runtime state invisibly outside committed turns or admitted adapter boundaries.

#### Scenario: Fake clock response is replayable
r[molten.testing.fixture_adapters.fake_clock]
- GIVEN a harness fixture for logical time
- WHEN an actor requests the clock effect
- THEN the request and fixture response are recorded as canonical effect records that can be replayed and compared by hash

#### Scenario: Mock adapter cannot bypass policy invisibly
r[molten.testing.fixture_adapters.no_invisible_mutation]
- GIVEN a test adapter with authority to provide fixture responses
- WHEN it changes visible runtime state or denies an operation
- THEN that decision is represented as an admitted effect, trace, receipt, or committed action visible to the harness report

### Requirement: Canonical oracles and matching
r[molten.testing.canonical_oracles] Test oracles MUST support exact canonical Preserves equality, deterministic Preserves pattern matching, trace predicates, receipt predicates, expected denial or failure classes, final state hashes, expected absence of side effects, and first-divergence expectations.

#### Scenario: Oracle compares trace predicate
r[molten.testing.canonical_oracles.trace_predicate]
- GIVEN a step expecting an actor turn to commit one assertion and emit one receipt
- WHEN the harness evaluates the step outcome
- THEN it compares canonical trace and receipt records rather than only checking rendered log text

### Requirement: First-divergence test diagnostics
r[molten.testing.first_divergence_reports] On mismatch, deterministic harness modes MUST stop at the first identified semantic divergence and report the divergence kind, expected and actual canonical hashes, suite/case/step id, handler profile, seed or effect-log position, relevant trace/receipt refs, and a redacted rendered diff when policy allows.

#### Scenario: Changed effect response reports first divergence
r[molten.testing.first_divergence_reports.effect_response]
- GIVEN a recorded suite whose replay receives a different storage or clock fixture response
- WHEN the harness detects the mismatch
- THEN it reports an effect-response divergence at the matching step and effect sequence before reporting downstream state differences

### Requirement: Canonical failure artifacts
r[molten.testing.canonical_failure_artifacts] The harness MUST emit canonical Preserves failure artifacts for preflight, execution, replay, validation, and export failures rather than relying on terminal stderr, JSON/JUnit rendering, or process exit status as normative failure evidence. Failure artifacts MUST identify phase, failure kind, message, relevant suite/report refs when available, first-divergence details when applicable, and diagnostics as canonical Preserves values.

#### Scenario: Failed run writes canonical failure evidence
r[molten.testing.canonical_failure_artifacts.run_failure]
- GIVEN a suite that fails because of an unknown actor, unsupported actor kind, resource budget exhaustion, or denied effect
- WHEN the harness is asked to write a report artifact
- THEN it writes a canonical `<harness-failure-v1 ...>` artifact with suite ref and diagnostic records, exits with failure, and does not rely on stderr as normative evidence

#### Scenario: Failure artifacts do not satisfy pass gates
r[molten.testing.canonical_failure_artifacts.not_pass_evidence]
- GIVEN a canonical failure artifact from a preflight, execution, replay, validation, or export failure
- WHEN a CI, admission, release, deterministic replay, or evidence gate requires a passing run report
- THEN the gate rejects the failure artifact as pass evidence while preserving it as normative diagnostic evidence

#### Scenario: Replay and validation failures keep first-divergence evidence
r[molten.testing.canonical_failure_artifacts.first_divergence]
- GIVEN a tampered report, missing effect-log entry, changed effect response, actor-registry mismatch, or state hash drift
- WHEN validation or replay fails
- THEN the failure artifact records the report ref, phase, failure kind, first divergent step when known, expected and actual refs or values, and detail diagnostics as canonical Preserves records

### Requirement: Gate receipts
r[molten.testing.gate_receipts] Successful pass-evidence gate decisions MUST emit canonical Preserves gate receipt artifacts rather than relying on terminal output or process exit status. Gate receipts MUST identify the admitted artifact, report ref, suite ref, final state ref, validation result, deterministic replay result, budget check evidence, actor-registry check evidence, and the individual gate checks that passed.

#### Scenario: Gate decision emits canonical receipt
r[molten.testing.gate_receipts.success]
- GIVEN a deterministic report or report repro bundle that validates and replays successfully
- WHEN `molten test gate check` accepts it as pass evidence
- THEN it emits a canonical `<gate-receipt-v1 "molten.harness.gate-receipt.v1" ...>` artifact with artifact refs and validation/replay/budget/actor-registry check evidence

### Requirement: Harness receipts and reports
r[molten.testing.run_receipts] The harness MUST emit receipt-backed run reports for suite start, step result, adapter fixture decisions, expected failures, known bugs, final status, and report export.

#### Scenario: CI can validate final report evidence
r[molten.testing.run_receipts.ci_validate]
- GIVEN a completed deterministic harness run
- WHEN CI validates the final report
- THEN the report references canonical trace records, child receipts, initial/final state hashes, profile identity, and status classification sufficient to reproduce or replay the run

### Requirement: Policy-gated test confidentiality
r[molten.testing.redaction_policy] The harness MUST gate running, reading, and exporting test reports through policy when suites or reports contain secrets, capabilities, external observations, exploit reproductions, or confidential trace data, and MUST apply redaction markers or encrypted refs where required.

#### Scenario: Secret fixture is redacted on export
r[molten.testing.redaction_policy.secret_export]
- GIVEN a test run that used a secret fixture or capability-bearing effect log
- WHEN a user exports the report without reveal authority
- THEN the exported report preserves canonical redaction markers or encrypted refs rather than exposing the secret bytes

### Requirement: Transcript, replay, chaos, property, and dogfood integration
r[molten.testing.integration_rails] The harness MUST integrate executable transcript stanzas, record/replay profiles, deterministic chaos profiles, Hegel property tests, Trellis predicate checks, and operator dogfood workflows under the same canonical evidence and report model.

#### Scenario: Transcript stanza becomes harness step
r[molten.testing.integration_rails.transcript_step]
- GIVEN an executable transcript with a Molten CLI stanza and expected trace pattern
- WHEN the transcript is run through the harness
- THEN the stanza is represented as a harness step with canonical inputs, observations, trace refs, receipts, and report status

#### Scenario: Property counterexample is replayable
r[molten.testing.integration_rails.property_counterexample]
- GIVEN a Hegel property test that finds and shrinks a counterexample
- WHEN the harness records the failing case
- THEN the generated input, shrink seed, runtime identity, trace refs, and final diagnostic are stored as canonical Preserves fixtures suitable for deterministic replay

### Requirement: Adapter conformance suites
r[molten.testing.adapter_conformance] The harness MUST provide adapter conformance suites for runtime adapters, including Iroh, Redb, Wasmtime/WASI, Steel, blob/chunk stores, typed storage, policy, resource, and fake network adapters, and MUST evaluate those adapters through the same canonical Preserves/effect request-response contract used by production runtime paths.

#### Scenario: Adapter conformance preserves effect evidence
r[molten.testing.adapter_conformance.effect_evidence]
- GIVEN an adapter implementation under conformance test
- WHEN the harness runs an admitted request, a denied request, and a failure response through the adapter
- THEN each request, response, denial, state change, trace, and receipt is represented as canonical Preserves evidence suitable for replay comparison

### Requirement: Cross-actor-kind interoperability suites
r[molten.testing.actor_kind_interop] The harness MUST test native Rust actors, Steel trusted-orchestration actors, Wasm component actors, adapter-backed actors, and remote-proxy actors through the same Molten envelope, Preserves dataspace assertion/retraction, Observe, admitted hostcall, policy, effect, trace, and receipt boundaries. Actor kind MUST be treated as an execution adapter detail rather than a separate communication semantic.

#### Scenario: Native actor communicates with Wasm actor
r[molten.testing.actor_kind_interop.native_wasm]
- GIVEN a native Rust actor and a Wasm component actor in the same deterministic harness run
- WHEN the native actor sends a message or assertion to the Wasm actor
- THEN delivery occurs through a canonical Molten envelope or Preserves dataspace value, admitted Wasm hostcalls, policy evidence, trace records, and receipts rather than direct adapter internals

#### Scenario: Wasm assertion is observed by Steel
r[molten.testing.actor_kind_interop.wasm_steel]
- GIVEN a Wasm actor with an admitted assertion hostcall and a Steel orchestration actor observing a matching pattern
- WHEN the Wasm actor asserts the value
- THEN the Steel actor observes the canonical Preserves assertion through the runtime API and the harness can replay the same observation deterministically

#### Scenario: Cross-kind send is denied without authority
r[molten.testing.actor_kind_interop.denied_cross_kind]
- GIVEN actors of different execution kinds without a matching send or assertion capability
- WHEN one actor attempts to communicate with the other
- THEN the runtime denies the action before delivery and records the denial in canonical trace and receipt evidence

### Requirement: System-layer behavior suites
r[molten.testing.system_layer_suites] The harness MUST support Molten system-layer suites for demand-driven services, dependency resolution, readiness/failure/completion assertions, logical supervision, restart and shutdown policy, capability-scoped service refs, assertion auto-retraction, policy admission, and deterministic replay. These suites MUST validate Molten's own Synit/SAM-inspired semantics without claiming Synit PID1, sturdyref, service-manager, wire-protocol, or configuration compatibility.

#### Scenario: Demand starts dependency-gated service
r[molten.testing.system_layer_suites.demand_start]
- GIVEN a service demand assertion for `worker` and a declared dependency on `network-ready`
- WHEN `network-ready` is not asserted
- THEN the harness observes that `worker` startup is withheld until the dependency assertion appears and the decision is traceable as canonical Preserves evidence

#### Scenario: Crash retracts service readiness and triggers supervision
r[molten.testing.system_layer_suites.crash_retract_restart]
- GIVEN a running service that asserted readiness and exposed a scoped service ref
- WHEN the service actor crashes or loses authority
- THEN the runtime auto-retracts readiness and dependent service refs, emits failure/supervision evidence, and applies the declared restart or degrade policy deterministically

#### Scenario: System-layer replay is stable across actor kinds
r[molten.testing.system_layer_suites.cross_kind_replay]
- GIVEN a system-layer suite whose services are implemented by native, Steel, Wasm, and adapter-backed actors
- WHEN the suite is rerun with the same artifacts, policy refs, profile, and seed or replay log
- THEN service demand, readiness, failure, restart, communication, traces, receipts, and final state hashes match canonically

### Requirement: Reproducibility bundles
r[molten.testing.repro_bundles] The harness MUST export minimal, policy-redacted reproducibility bundles for deterministic or recorded failures, including suite/case/step refs, artifact dependency closure, initial snapshot or fixture refs, schema and policy refs, handler profile, seed or effect-log segment, relevant trace and receipt refs, final or divergent state hashes, and first-divergence diagnostics.

#### Scenario: Developer reruns exported failure
r[molten.testing.repro_bundles.rerun]
- GIVEN a failed deterministic harness run and an exported repro bundle
- WHEN another developer imports and reruns the bundle with matching runtime artifacts
- THEN the harness reconstructs the declared initial state and reaches the same first-divergence boundary without relying on ambient local state

### Requirement: Counterexample shrinking rail
r[molten.testing.counterexample_shrinking] Property-test failures MUST record generation seed, shrink path, final shrunk Preserves fixture, replay identity, traces, receipts, and diagnostics so the counterexample can become a deterministic regression case.

#### Scenario: Shrunk property failure becomes replay case
r[molten.testing.counterexample_shrinking.replay_case]
- GIVEN a Hegel property failure that shrinks to a smaller input
- WHEN the harness stores the failure report
- THEN the shrunk input and replay identity are available as a canonical fixture that can be run without invoking the generator

### Requirement: Negative and security suites
r[molten.testing.negative_security_suites] The harness MUST include first-class negative and security suites for denied capabilities, revoked authority, malformed envelopes, invalid or noncanonical Preserves values, tampered content refs, invalid receipts, policy denial, resource exhaustion, replay-protection failures, redaction leaks, confused-deputy attempts, and unauthorized report export.

#### Scenario: Tampered content is denied before side effects
r[molten.testing.negative_security_suites.tampered_content]
- GIVEN a harness case with a content ref whose bytes do not match its declared hash
- WHEN the runtime attempts to admit or fetch the content
- THEN the action is denied before actor delivery or adapter side effects and the denial is recorded in trace and receipt evidence

### Requirement: Upgrade and migration replay
r[molten.testing.upgrade_replay] The harness MUST replay old canonical traces, reports, snapshots, schemas, policies, and artifact fixtures against new runtime versions and MUST require stable replay, explicit migration receipts, or explicit compatibility diagnostics for intentional incompatible changes.

#### Scenario: Runtime upgrade explains trace drift
r[molten.testing.upgrade_replay.trace_drift]
- GIVEN a golden trace from an earlier compatible runtime version
- WHEN a newer runtime produces a different canonical trace for the same replay identity
- THEN the harness reports either a replay failure or an approved migration/compatibility receipt explaining the change

### Requirement: Runtime-boundary coverage
r[molten.testing.boundary_coverage] Harness reports MUST be able to summarize coverage by runtime boundary, including envelope routes, dataspace semantics, policy gates, effect handlers, receipts, traces, storage paths, resource decisions, replay branches, adapter boundaries, and confidentiality paths, rather than only source-line coverage.

#### Scenario: Report identifies unexercised gate
r[molten.testing.boundary_coverage.unexercised_gate]
- GIVEN a suite that exercises actor sends but no policy denials
- WHEN the harness renders boundary coverage
- THEN the report identifies the policy-denial boundary as unexercised even if source-line coverage is high

### Requirement: Deterministic multi-peer simulation
r[molten.testing.deterministic_multipeer] The harness MUST support deterministic multi-peer simulation where peer delivery, partitions, drops, reorders, reconnects, logical clocks, resource limits, gossip, docs, and blob observations are driven by seeded profiles or recorded logs.

#### Scenario: Partition replay is stable
r[molten.testing.deterministic_multipeer.partition_replay]
- GIVEN a multi-peer suite with a seeded partition and reconnect schedule
- WHEN the harness runs the suite twice with the same artifacts, profile, and seed
- THEN peer-visible observations, traces, receipts, and final state hashes match canonically

### Requirement: Resource and performance regression rail
r[molten.testing.resource_regression] The harness MUST support deterministic budget assertions for turns, scheduler steps, mailbox depth, assertion count, effect calls, blob/storage/network bytes, trace bytes, Wasm fuel, Steel/native checkpoints, and job-stage resources. Wall-clock timing MAY be reported as advisory metadata but MUST NOT be the normative deterministic gate.

#### Scenario: Effect-count regression fails deterministically
r[molten.testing.resource_regression.effect_count]
- GIVEN a suite with an expected maximum effect-call budget
- WHEN a runtime change emits additional effect requests beyond the budget
- THEN the harness fails the run with a resource-regression diagnostic tied to canonical effect records

### Requirement: Golden canonical traces
r[molten.testing.golden_traces] The harness MUST support versioned golden canonical trace, receipt, and state-hash artifacts for important runtime stories, and changes to those artifacts MUST be reviewed with receipts identifying whether the change is schema-driven, policy-driven, migration-driven, or a bug fix.

#### Scenario: Golden update requires receipt
r[molten.testing.golden_traces.update_receipt]
- GIVEN a proposed update to a golden trace artifact
- WHEN the update is admitted
- THEN the harness records a receipt that identifies the old and new trace refs, reviewer or policy authority, and reason class

### Requirement: Flake prevention policy
r[molten.testing.flake_prevention] CI, admission, release, and upgrade gates MUST reject flaky or ambient-state-dependent tests as evidence. A gated harness run MUST be deterministic, replayed, recorded for replay, or explicitly marked exploratory/non-replayable and excluded from deterministic evidence.

#### Scenario: Non-replayable exploratory run cannot satisfy gate
r[molten.testing.flake_prevention.exploratory_excluded]
- GIVEN an exploratory harness run that observes nondeterministic external state without recording a replay log
- WHEN CI evaluates deterministic evidence requirements
- THEN the run is excluded from satisfying the gate even if its rendered status is pass

### Requirement: Harness CLI surface
r[molten.testing.cli_surface] The system MUST expose a CLI surface for listing suites, running deterministic suites, replaying recorded runs, showing canonical reports, exporting policy-redacted report views, and exporting reproducibility bundles.

#### Scenario: Developer runs a local suite
r[molten.testing.cli_surface.local_run]
- GIVEN a path or artifact ref for a deterministic local test suite
- WHEN a developer runs the harness CLI for that suite
- THEN the CLI executes the suite through the fresh local runner and prints a rendered summary whose report id resolves to canonical Preserves evidence
