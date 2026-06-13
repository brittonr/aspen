# Testing Harness Specification

## Purpose

Defines the `testing-harness` capability.

## Requirements

### Requirement: Export profiles are explicit
r[molten.testing.redacted_repro_export_profiles.profile_schema] Repro export MUST require an explicit confidentiality profile whenever sensitive markers are present.

#### Scenario: Default profile remains fail-closed
- GIVEN a report containing `<secret ...>`
- WHEN repro export uses the default profile
- THEN export fails closed before writing a sealed pass bundle

#### Scenario: Redacted diagnostic profile emits transform evidence
- GIVEN a report containing sensitive markers
- WHEN repro export uses `redacted-diagnostic`
- THEN the output bundle contains deterministic redaction markers
- AND the bundle contains a redaction transform receipt bound to the source report and output bundle
- AND the bundle is marked diagnostic-only unless policy says otherwise

### Requirement: Transform receipts bind all redactions
r[molten.testing.redacted_repro_export_profiles.transform_receipt] Redaction transform receipts MUST bind the source report ref, suite ref, redaction policy ref, profile, transform manifest, and output bundle ref.

#### Scenario: Stale transform receipt is rejected
- GIVEN a redacted bundle with a transform receipt from another report
- WHEN verify, unpack, or gate checks run
- THEN the bundle fails closed with a transform binding diagnostic

#### Scenario: Missed sensitive marker is rejected
- GIVEN a redacted bundle whose transform manifest does not cover every sensitive marker
- WHEN verify, unpack, or gate checks run
- THEN the bundle fails closed before materializing private content

### Requirement: Encrypted refs require validation and reveal receipts
r[molten.testing.redacted_repro_export_profiles.encrypted_ref_validation] `<encrypted-ref ...>` values MUST remain fail-closed unless encryption metadata, recipient policy, and reveal receipts validate.

#### Scenario: Malformed encrypted ref is rejected
- GIVEN a redacted or encrypted bundle containing a malformed `<encrypted-ref ...>`
- WHEN verify or unpack runs
- THEN the bundle fails closed

#### Scenario: Authorized reveal materializes private content
- GIVEN an encrypted-private bundle and a matching reveal receipt
- WHEN unpack runs with reveal authority
- THEN only authorized private material is materialized
- AND the reveal receipt is written beside the unpacked bundle evidence

### Requirement: Reveal receipts bind encrypted repro refs directly
r[molten.testing.repro_reveal_encrypted_ref_binding.receipt_field] Reveal receipts used for encrypted-private repro unpack MUST carry an explicit encrypted-ref binding and a corresponding binding check.

#### Scenario: Bound reveal receipt is accepted
- GIVEN an encrypted-private repro bundle with an encrypted ref
- WHEN unpack receives a passing reveal receipt bound to that exact encrypted ref
- THEN unpack may materialize the authorized private repro evidence

#### Scenario: Legacy generic reveal receipt is not enough for repro unpack
- GIVEN an encrypted-private repro bundle and a passing legacy reveal receipt with no encrypted-ref binding
- WHEN unpack runs with that reveal receipt
- THEN unpack fails closed before materializing private content

### Requirement: Unpack matches exact bundle encrypted refs
r[molten.testing.repro_reveal_encrypted_ref_binding.unpack_match] Repro unpack MUST authorize encrypted-private material only by exact encrypted-ref ids present in the bundle.

#### Scenario: Stale reveal binding is rejected
- GIVEN a reveal receipt whose secret or commitment ref matches a bundle encrypted ref but whose encrypted-ref field names another ref
- WHEN encrypted-private repro unpack runs
- THEN unpack fails closed with a stale or unrelated reveal diagnostic

### Requirement: Reveal coverage remains complete and evidence-only
r[molten.testing.repro_reveal_encrypted_ref_binding.partial_coverage_denial] Repro unpack MUST fail closed unless every encrypted ref in the bundle has a passing exact-bound reveal receipt.

#### Scenario: Partial reveal coverage is rejected
- GIVEN an encrypted-private repro bundle with one or more encrypted refs
- WHEN any encrypted ref lacks a passing exact-bound reveal receipt
- THEN unpack fails closed before writing private material

r[molten.testing.repro_reveal_encrypted_ref_binding.evidence_only] Reveal receipt bindings MUST NOT make encrypted-private repro bundles gate-preserving pass evidence.

#### Scenario: Reveal does not grant pass-gate evidence
- GIVEN an encrypted-private repro bundle with complete reveal receipts
- WHEN pass gate verification evaluates the bundle
- THEN the bundle remains requires-reveal private evidence and is not accepted as gate-preserving pass evidence

### Requirement: Report repro bundles are sealed pass artifacts
r[molten.testing.sealed_repro_bundles.schema] Exported report repro bundles MUST include seal metadata, an embedded report gate receipt, and artifact refs for the report evidence required to validate and replay the run.

#### Scenario: Exported report bundle contains a seal
- GIVEN a deterministic report that validates and gates successfully
- WHEN `molten test repro export` exports the report
- THEN `refs.preserves` contains `<repro-seal ...>`, an embedded `<gate-receipt-v1 ...>`, and refs for the report, suite, actor registry, effect log, policy gate, capability gate, budget gate, and gate receipt

### Requirement: Sealed bundles validate embedded refs fail-closed
r[molten.testing.sealed_repro_bundles.validation] Parsing or gate checking a sealed bundle MUST recompute embedded report refs and receipt refs instead of trusting the bundle fields.

#### Scenario: Tampered embedded report fails
- GIVEN a sealed report repro bundle
- WHEN the embedded report is changed after sealing
- THEN gate checking the bundle fails closed before accepting it as pass evidence

#### Scenario: Tampered embedded receipt fails
- GIVEN a sealed report repro bundle
- WHEN the embedded gate receipt is changed after sealing
- THEN gate checking the bundle fails closed with a seal or receipt diagnostic

#### Scenario: Mismatched suite ref fails
- GIVEN a sealed report repro bundle
- WHEN the bundle suite ref or suite artifact ref no longer matches the embedded report
- THEN gate checking the bundle fails closed

### Requirement: Bundle gates recompute the report gate receipt
r[molten.testing.sealed_repro_bundles.gate] A sealed bundle MUST satisfy pass evidence only if its embedded report gate receipt exactly matches the receipt recomputed from the embedded report.

#### Scenario: Sealed bundle gates as repro-bundle artifact
- GIVEN a valid sealed report repro bundle
- WHEN `molten test gate check refs.preserves` runs
- THEN the embedded report receipt is validated
- AND the command emits a new gate receipt whose artifact kind is `repro-bundle`

### Requirement: Failure repro bundles remain diagnostics only
r[molten.testing.sealed_repro_bundles.failure_diagnostics] Failure repro bundles MUST NOT satisfy pass evidence gates.

#### Scenario: Failure bundle rejected by pass gate
- GIVEN a failure repro bundle
- WHEN `molten test gate check refs.preserves` runs
- THEN the gate rejects it as diagnostic evidence only

### Requirement: Repro export writes sealed bundle files
r[molten.testing.sealed_repro_bundles.export] The harness CLI MUST write sealed report repro bundle files, including the embedded report gate receipt, when exporting a valid deterministic report.

#### Scenario: Export writes receipt artifact
- GIVEN a deterministic report that passes validation and report gating
- WHEN `molten test repro export` writes a bundle directory
- THEN the directory contains the sealed refs file and embedded gate receipt artifact
- AND the refs file binds the report, suite, and receipt content refs

### Requirement: Sealed bundle regressions cover tamper cases
r[molten.testing.sealed_repro_bundles.negative_tests] Sealed bundle tests SHOULD cover tampered reports, tampered embedded receipts, mismatched suite refs, and diagnostic-only failure bundles.

#### Scenario: Tamper test fails before pass evidence
- GIVEN a sealed bundle negative fixture with one tampered embedded artifact
- WHEN the bundle gate is evaluated in tests
- THEN the gate fails closed before emitting pass evidence

### Requirement: Sealed bundle CLI contracts are documented
r[molten.testing.sealed_repro_bundles.docs] User-facing documentation SHOULD describe sealed repro export, embedded receipt validation, and the diagnostic-only status of failure bundles.

#### Scenario: Operator follows sealed export docs
- GIVEN an operator reading the repro bundle CLI documentation
- WHEN they export and gate a deterministic report bundle
- THEN the documented commands identify the sealed refs file and embedded receipt evidence required for pass validation

### Requirement: Sealed repro verify emits canonical receipts
r[molten.testing.sealed_repro_verify_unpack.verify_cli] The harness CLI MUST provide a sealed repro verification command that emits canonical verification receipts for valid sealed report bundles.

#### Scenario: Valid sealed bundle verifies
- GIVEN a sealed report repro bundle
- WHEN `molten test repro verify refs.preserves` runs
- THEN it validates the embedded report, deterministic replay, and embedded report gate receipt
- AND emits `<repro-verify-receipt-v1 ...>` with bundle, report, suite, and gate receipt refs

#### Scenario: Tampered bundle fails verify
- GIVEN a sealed report repro bundle whose embedded report, seal, artifact refs, or gate receipt has been modified
- WHEN `molten test repro verify refs.preserves` runs
- THEN verification fails closed and can emit a canonical failure artifact

### Requirement: Sealed repro unpack materializes verified contents
r[molten.testing.sealed_repro_verify_unpack.unpack_cli] The harness CLI MUST provide an unpack command that materializes only verified sealed report bundles.

#### Scenario: Valid sealed bundle unpacks
- GIVEN a valid sealed report repro bundle
- WHEN `molten test repro unpack refs.preserves --out DIR` runs
- THEN DIR contains `refs.preserves`, `report.preserves`, `suite.preserves`, `gate-receipt.preserves`, `verify-receipt.preserves`, `summary.txt`, and `commands.txt`
- AND the unpacked report and receipt refs match the sealed bundle

### Requirement: Diagnostic bundles remain non-pass evidence
r[molten.testing.sealed_repro_verify_unpack.diagnostic_only] Failure repro bundles and unsealed legacy bundles MUST NOT satisfy sealed verify/unpack commands.

#### Scenario: Failure bundle rejected by verify and unpack
- GIVEN a failure repro bundle
- WHEN `molten test repro verify` or `molten test repro unpack` runs
- THEN the command fails closed with a diagnostic-only error and optional canonical failure artifact

### Requirement: Verification receipts are parseable and summarizable
r[molten.testing.sealed_repro_verify_unpack.verify_receipt] Repro verification receipts MUST be parseable, summarizable, and suitable for binding bundle, report, suite, and gate receipt refs in later evidence.

#### Scenario: Verification receipt summary names refs
- GIVEN a passing `<repro-verify-receipt-v1 ...>`
- WHEN the receipt is shown or parsed by the harness
- THEN the summary includes the bundle ref, report ref, suite ref, gate receipt ref, and verification status

### Requirement: Verify and unpack fail closed on invalid bundles
r[molten.testing.sealed_repro_verify_unpack.fail_closed] Verify and unpack commands MUST reject tampered, unsealed, missing, or diagnostic-only repro bundles before materializing pass evidence.

#### Scenario: Unsealed bundle is not unpacked
- GIVEN a legacy unsealed bundle
- WHEN `molten test repro unpack` is requested
- THEN the command fails closed
- AND no verified output directory is materialized as pass evidence

### Requirement: Verify and unpack behavior has CLI coverage
r[molten.testing.sealed_repro_verify_unpack.tests] The harness SHOULD have CLI or integration tests for valid verify, valid unpack, tamper rejection, and failure-bundle rejection.

#### Scenario: CLI test covers verified unpack
- GIVEN a deterministic report exported as a sealed bundle
- WHEN the CLI test verifies and unpacks the bundle
- THEN the unpacked refs, report, suite, gate receipt, and verification receipt match the sealed refs

### Requirement: Verify and unpack commands are documented
r[molten.testing.sealed_repro_verify_unpack.docs] User-facing documentation SHOULD describe repro verify and unpack commands, verification receipt outputs, and fail-closed diagnostics.

#### Scenario: Operator follows unpack docs
- GIVEN an operator reading repro verify and unpack documentation
- WHEN they unpack a sealed bundle
- THEN the documented commands require verification before materializing bundle contents

### Requirement: Sealed bundles include redaction evidence
r[molten.testing.sealed_repro_redaction_preflight.policy] Sealed report repro bundles MUST include canonical redaction policy evidence and redaction gate evidence before they can satisfy pass evidence gates.

#### Scenario: Normal sealed bundle includes redaction preflight
- GIVEN a deterministic report without sensitive markers
- WHEN `molten test repro export` exports a sealed bundle
- THEN the bundle contains `<redaction-policy-v1 ...>` and `<redaction-gate-v1 ...>` evidence
- AND gate/verify/unpack recompute the same redaction refs

### Requirement: Sensitive markers fail closed
r[molten.testing.sealed_repro_redaction_preflight.scan] Sealed report repro export MUST reject reports whose canonical Preserves values contain sensitive marker records.

#### Scenario: Secret marker blocks export
- GIVEN a deterministic report whose suite, observation, effect log, or report evidence contains `<secret ...>`
- WHEN sealed repro export runs
- THEN export fails closed with a redaction preflight diagnostic

#### Scenario: Confidential markers block export
- GIVEN a report containing `<confidential ...>`, `<credential ...>`, `<private ...>`, or `<encrypted-ref ...>`
- WHEN sealed repro export runs
- THEN export fails closed until explicit redaction/encryption validation exists

### Requirement: Tampered or missing redaction evidence fails gates
r[molten.testing.sealed_repro_redaction_preflight.validation] Bundle gate checks MUST reject missing, stale, or tampered redaction policy/gate evidence.

#### Scenario: Tampered redaction gate fails
- GIVEN a sealed report repro bundle
- WHEN its redaction gate evidence is changed after sealing
- THEN parsing, verification, unpacking, or gate checking rejects the bundle

#### Scenario: Unsealed report bundle no longer satisfies pass gate
- GIVEN a legacy report repro bundle without redaction preflight evidence
- WHEN `molten test gate check` runs on it
- THEN the gate fails closed because redaction preflight evidence is missing

### Requirement: Redaction gates are bound to report evidence
r[molten.testing.sealed_repro_redaction_preflight.gate] Redaction gate evidence MUST bind the embedded report ref, suite ref, policy ref, sensitive-marker scan result, and final allow-or-deny decision before a sealed bundle can satisfy pass gates.

#### Scenario: Redaction gate recomputes clean report refs
- GIVEN a clean deterministic report exported as a sealed bundle
- WHEN bundle verification recomputes redaction evidence
- THEN the recomputed redaction gate ref matches the bundle ref
- AND the gate decision remains pass for the same report and suite refs

### Requirement: Unsealed report bundles are rejected from pass gates
r[molten.testing.sealed_repro_redaction_preflight.unsealed_rejection] Pass-evidence gates MUST reject report repro bundles that do not include redaction preflight evidence.

#### Scenario: Legacy bundle fails pass evidence gate
- GIVEN a legacy report repro bundle without redaction policy and gate refs
- WHEN the bundle is used as pass evidence
- THEN the gate rejects it before trusting the embedded report or receipt

### Requirement: Redaction preflight has negative coverage
r[molten.testing.sealed_repro_redaction_preflight.tests] Redaction preflight tests SHOULD cover sensitive markers, missing evidence, stale evidence, and tampered redaction gate refs.

#### Scenario: Sensitive marker test denies export
- GIVEN a report fixture containing a sensitive marker record
- WHEN sealed repro export is exercised in tests
- THEN the export fails closed with a redaction diagnostic

### Requirement: Redaction preflight commands are documented
r[molten.testing.sealed_repro_redaction_preflight.docs] User-facing documentation SHOULD describe redaction policy evidence, redaction gate evidence, sensitive-marker failures, and legacy unsealed bundle rejection.

#### Scenario: Operator follows redaction docs
- GIVEN an operator reading repro redaction documentation
- WHEN they export a pass-evidence bundle
- THEN the documented evidence includes redaction policy and gate refs before verification or unpacking

### Requirement: Harness gates emit generic replay evidence
r[molten.determinism.harness_generic_replay.emit] Harness pass gates SHOULD emit generic `deterministic-replay-verify-v1` evidence from the deterministic replay comparison used for gate acceptance.

#### Scenario: Gate receipt embeds replay verification
- GIVEN a harness report that validates and replays successfully
- WHEN a gate receipt is emitted
- THEN the receipt includes generic replay verification evidence with a pass decision
- AND the generic receipt binds expected report, actual report, and final-state refs

### Requirement: Gate artifact refs bind generic replay evidence
r[molten.determinism.harness_generic_replay.artifact_ref] Gate receipts SHOULD list the generic deterministic replay verification receipt ref as an artifact ref.

#### Scenario: Replay verify ref is indexed
- GIVEN a gate receipt with embedded generic replay evidence
- WHEN artifact refs are inspected
- THEN an artifact ref with kind `deterministic-replay-verify` points to the embedded replay verification value

### Requirement: Gate parsing validates embedded generic replay evidence
r[molten.determinism.harness_generic_replay.parse] Gate receipt parsing MUST validate that embedded generic replay evidence is a pass receipt, has no divergence, and binds the same report and final-state refs as the gate replay block.

#### Scenario: Tampered generic replay receipt is rejected
- GIVEN a gate receipt whose embedded generic replay receipt has a changed report ref, final-state ref, decision, or divergence
- WHEN the gate receipt is parsed
- THEN parsing fails closed before accepting the gate receipt

### Requirement: Harness generic replay evidence is tested
r[molten.determinism.harness_generic_replay.tests] Molten SHOULD test that harness gate receipts contain and validate generic replay evidence while preserving existing replay checks.

#### Scenario: Generic replay evidence remains evidence-only
- GIVEN a gate receipt with a generic replay verification receipt
- WHEN tests inspect the gate receipt
- THEN they find the generic replay evidence and artifact ref
- AND report validation, policy, capability, resource, chain, turn journal, and source-gate checks remain required separately

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

### Requirement: Evidence-bearing suites require explicit budget fixtures
r[molten.testing.mandatory_budget.explicit_fixture] Evidence-bearing harness suites MUST include an explicit budget fixture or equivalent resource-policy proof refs. Omitted budget fixtures MUST NOT be normalized to default resource policy for execution, validation, or pass-evidence gates.

#### Scenario: Omitted budget fails execution
r[molten.testing.mandatory_budget.explicit_fixture.omitted]
- GIVEN a harness suite with explicit actor registry, explicit capabilities, actor steps, and no `<budget-v1 ...>` fixture
- WHEN the evidence-bearing local runner attempts to execute the suite
- THEN the runner rejects it before runtime turns, admission decisions, ambient effect requests, or report generation occur

#### Scenario: Explicit standard budget is valid
r[molten.testing.mandatory_budget.explicit_fixture.standard]
- GIVEN a harness suite with `<budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>`
- WHEN the suite stays within those limits
- THEN the budget fixture is explicit and may satisfy pass-evidence gates

#### Scenario: Explicit tight budget remains resource divergence
r[molten.testing.mandatory_budget.explicit_fixture.tight]
- GIVEN a harness suite with an explicit tight budget
- WHEN execution exceeds the declared resource limit
- THEN the runner reports deterministic `resource` divergence with expected, actual, and step diagnostics rather than treating the suite as malformed

### Requirement: Report validation rejects default resource policy
r[molten.testing.mandatory_budget.validation] Report validation MUST reject embedded suites that omitted explicit budget evidence, even if the report contains default `<budget-v1 ...>` evidence produced by an older runner.

#### Scenario: Legacy report with default budget fails validation
r[molten.testing.mandatory_budget.validation.legacy]
- GIVEN a report produced by an older runner whose embedded suite omitted the budget fixture
- WHEN `molten test report validate` evaluates the report
- THEN validation fails closed with missing explicit budget fixture diagnostics

#### Scenario: Budget usage still matches explicit evidence
r[molten.testing.mandatory_budget.validation.usage]
- GIVEN a report whose embedded suite declares an explicit budget fixture
- WHEN report usage counts differ from observations, effect-log entries, event counts, canonical report bytes, or declared limits
- THEN validation rejects the report with budget evidence diagnostics

### Requirement: Pass-evidence receipts prove no default resource policy
r[molten.testing.mandatory_budget.gate_checks] Successful pass-evidence gate receipts MUST include checks proving the accepted report used an explicit budget fixture and no default resource policy.

#### Scenario: Receipt includes explicit budget checks
r[molten.testing.mandatory_budget.gate_checks.receipt]
- GIVEN a deterministic report with an explicit budget fixture that validates and replays successfully
- WHEN `molten test gate check` emits a pass receipt
- THEN the receipt includes `explicit-budget-fixture` and `no-default-resource-policy` checks in addition to budget, actor-registry, capability, policy, admission, effect-log, and replay checks

### Requirement: Examples declare budgets
r[molten.testing.mandatory_budget.examples] Repository examples and positive harness tests MUST declare explicit budget fixtures. Negative resource tests MUST use explicit tight budgets, not omitted budgets, unless the test specifically targets omitted-budget failure.

#### Scenario: Two-actor example declares budget
r[molten.testing.mandatory_budget.examples.two_actor]
- GIVEN the repository two-actor example suite
- WHEN it is run through the harness and gated as pass evidence
- THEN the suite includes an explicit budget fixture covering step, effect, event, and report byte limits

### Requirement: Future resource policy evidence remains explicit
r[molten.testing.mandatory_budget.basalt_resource_policy] Future Nickel/Basalt resource policy integration MUST preserve the invariant that missing resource-policy evidence fails closed. Nickel policy snapshots, Basalt receipts, resource profiles, and budget refs MAY replace the first local static fixture, but they MUST be explicit and bound to run identity.

#### Scenario: Missing future resource proof fails closed
r[molten.testing.mandatory_budget.basalt_resource_policy.missing]
- GIVEN a future evidence-bearing report whose resource policy comes from a Nickel/Basalt proof ref
- WHEN that proof ref or receipt is omitted
- THEN validation rejects the report rather than treating missing resource policy as the default budget

### Requirement: Evidence-bearing suites require explicit capability fixtures
r[molten.testing.mandatory_capabilities.explicit_fixture] Evidence-bearing harness suites MUST include an explicit capability fixture or equivalent authority proof refs. Omitted capability fixtures MUST NOT be normalized to implicit authority for execution, validation, or pass-evidence gates.

#### Scenario: Omitted fixture fails execution
r[molten.testing.mandatory_capabilities.explicit_fixture.omitted]
- GIVEN a harness suite with actor steps but no `<capabilities-v1 ...>` fixture
- WHEN the evidence-bearing local runner attempts to execute the suite
- THEN the runner rejects it before any runtime turn or ambient effect request occurs

#### Scenario: Explicit empty fixture is valid authority context
r[molten.testing.mandatory_capabilities.explicit_fixture.empty]
- GIVEN a harness suite with `<capabilities-v1 "molten.harness.capabilities.v1" []>`
- WHEN a step requests a send, assertion, observation, or effect
- THEN the request is denied through normal admission evidence rather than rejected as malformed suite evidence

### Requirement: Report validation rejects implicit authority
r[molten.testing.mandatory_capabilities.validation] Report validation MUST reject embedded suites that omitted explicit capability evidence, even if the report contains a capability gate record over a compatibility/default context.

#### Scenario: Legacy report with default authority fails validation
r[molten.testing.mandatory_capabilities.validation.legacy]
- GIVEN a report produced by an older runner whose embedded suite omitted capability fixtures
- WHEN `molten test report validate` evaluates the report
- THEN validation fails closed with missing explicit capability fixture diagnostics

### Requirement: Pass-evidence receipts prove no implicit authority
r[molten.testing.mandatory_capabilities.gate_checks] Successful pass-evidence gate receipts MUST include checks proving the accepted report used explicit capability evidence and no implicit authority default.

#### Scenario: Receipt includes explicit authority checks
r[molten.testing.mandatory_capabilities.gate_checks.receipt]
- GIVEN a deterministic report with explicit capability grants that validates and replays successfully
- WHEN `molten test gate check` emits a pass receipt
- THEN the receipt includes `explicit-capability-fixture` and `no-implicit-authority` checks in addition to capability context, grant, denial, authority binding, policy, admission, budget, actor-registry, effect-log, and replay checks

### Requirement: Examples use least-privilege grants
r[molten.testing.mandatory_capabilities.examples] Repository examples and positive harness tests MUST declare explicit least-privilege grants for the actions they expect to allow. Negative authority tests MUST use explicit empty fixtures or missing grants, not omitted fixtures.

#### Scenario: Two-actor example declares grants
r[molten.testing.mandatory_capabilities.examples.two_actor]
- GIVEN the repository two-actor example suite
- WHEN it is run through the harness and gated as pass evidence
- THEN the suite includes explicit grants for observe, assert, send, clock, random, and retract actions used by the test

### Requirement: Future Basalt/UCAN keeps explicit authority invariant
r[molten.testing.mandatory_capabilities.basalt_ucan_invariant] Future Basalt/UCAN authority proof integration MUST preserve the invariant that missing authority evidence fails closed. Proof bundles, caveats, revocation evidence, and authority receipts MAY replace local static grants, but they MUST be explicit and bound to run identity.

#### Scenario: Missing future proof fails closed
r[molten.testing.mandatory_capabilities.basalt_ucan_invariant.missing]
- GIVEN a future evidence-bearing report whose admission decision depends on a UCAN proof bundle
- WHEN the proof bundle or Basalt receipt ref is omitted
- THEN validation rejects the report rather than treating missing proof evidence as ambient authority

### Requirement: Evidence-bearing suites require explicit actor registries
r[molten.testing.mandatory_actor_registry.explicit_fixture] Evidence-bearing harness suites MUST include an explicit actor registry fixture or equivalent actor/executor proof refs. Omitted actor registries MUST NOT be inferred from steps, capability grants, policy rules, observations, or runner defaults for execution, validation, or pass-evidence gates.

#### Scenario: Omitted registry fails execution
r[molten.testing.mandatory_actor_registry.explicit_fixture.omitted]
- GIVEN a harness suite with actor-referencing steps, explicit capabilities, and no `<actor-registry-v1 ...>` fixture
- WHEN the evidence-bearing local runner attempts to execute the suite
- THEN the runner rejects it before actor executor setup, runtime turns, admission decisions, or ambient effect requests occur

#### Scenario: Explicit empty registry is valid only for empty actor use
r[molten.testing.mandatory_actor_registry.explicit_fixture.empty]
- GIVEN a harness suite with `<actor-registry-v1 "molten.harness.actor-registry.v1" []>`
- WHEN the suite contains no actor-referencing steps or evidence
- THEN the registry fixture is explicit and may satisfy the registry preflight
- BUT WHEN any step references an actor absent from the explicit registry
- THEN normal unknown-actor preflight rejects the suite

### Requirement: Report validation rejects inferred actor registries
r[molten.testing.mandatory_actor_registry.validation] Report validation MUST reject embedded suites that omitted explicit actor registry evidence, even if the report contains actor-registry evidence inferred by an older runner.

#### Scenario: Legacy report with inferred actors fails validation
r[molten.testing.mandatory_actor_registry.validation.legacy]
- GIVEN a report produced by an older runner whose embedded suite omitted the actor registry fixture
- WHEN `molten test report validate` evaluates the report
- THEN validation fails closed with missing explicit actor registry diagnostics

#### Scenario: Report actors must match explicit registry
r[molten.testing.mandatory_actor_registry.validation.mismatch]
- GIVEN a report whose embedded suite declares an explicit actor registry
- WHEN observations, effect records, admission requests, or final state mention an actor not present in that registry
- THEN validation rejects the report rather than accepting an inferred actor

### Requirement: Executor selection is a fail-closed boundary
r[molten.testing.mandatory_actor_registry.executor_boundary] Actor registry entries MUST bind actor ids to executor kinds, and evidence-bearing execution MUST NOT silently coerce unsupported or unreviewed kinds to native execution. Unsupported Steel, Wasm, adapter, and remote-proxy actors MUST fail until their executor boundary evidence is implemented and reviewed.

#### Scenario: Unsupported kind cannot fall back to native
r[molten.testing.mandatory_actor_registry.executor_boundary.unsupported]
- GIVEN an explicit actor registry containing `<actor "a" "wasm">` before Wasm executor boundary evidence is supported
- WHEN the evidence-bearing local runner attempts to execute a step for actor `a`
- THEN the runner rejects the suite before the step executes and does not run actor `a` as native

#### Scenario: Future executor evidence is explicit
r[molten.testing.mandatory_actor_registry.executor_boundary.future]
- GIVEN a future Steel, Wasm, adapter, or remote-proxy actor kind
- WHEN it participates in deterministic pass evidence
- THEN its registry entry is bound to explicit executor manifest, policy, replay, or exclusion evidence rather than runner defaults

### Requirement: Pass-evidence receipts prove no inferred actors
r[molten.testing.mandatory_actor_registry.gate_checks] Successful pass-evidence gate receipts MUST include checks proving the accepted report used an explicit actor registry, no inferred actors, and a reviewed executor boundary.

#### Scenario: Receipt includes explicit registry checks
r[molten.testing.mandatory_actor_registry.gate_checks.receipt]
- GIVEN a deterministic report with an explicit actor registry that validates and replays successfully
- WHEN `molten test gate check` emits a pass receipt
- THEN the receipt includes `explicit-actor-registry`, `no-inferred-actors`, and `executor-boundary` checks in addition to actor-registry, capability, policy, admission, budget, effect-log, and replay checks

### Requirement: Examples declare actor registries
r[molten.testing.mandatory_actor_registry.examples] Repository examples and positive harness tests MUST declare explicit actor registries for every actor they expect to use. Negative actor tests MUST use explicit registries with missing actors or unsupported kinds, not omitted registries, unless the test specifically targets omitted-registry failure.

#### Scenario: Two-actor example declares registry
r[molten.testing.mandatory_actor_registry.examples.two_actor]
- GIVEN the repository two-actor example suite
- WHEN it is run through the harness and gated as pass evidence
- THEN the suite includes explicit native actor entries for `consumer` and `producer`

### Requirement: Future executor evidence remains explicit
r[molten.testing.mandatory_actor_registry.future_executor_evidence] Future Steel, Wasm, adapter, and remote-proxy executor integration MUST preserve the invariant that missing executor-boundary evidence fails closed. Executor manifests, hostcall capabilities, adapter contracts, remote identity refs, non-replayable exclusions, and simulation receipts MAY replace the first local native-only executor check, but they MUST be explicit and bound to run identity.

#### Scenario: Missing future executor proof fails closed
r[molten.testing.mandatory_actor_registry.future_executor_evidence.missing]
- GIVEN a future evidence-bearing report whose actor registry includes a non-native executor kind
- WHEN the required executor manifest or boundary receipt is omitted
- THEN validation rejects the report rather than treating missing executor evidence as native execution authority

### Requirement: Admission evidence records are mandatory
r[molten.testing.admission_evidence.records] Evidence-bearing harness reports MUST record exactly one canonical admission decision event for every step observation. The admission decision event MUST precede semantic runtime trace records, effect request/response records, rollback records, or output records for that step.

#### Scenario: Missing admission decision fails validation
r[molten.testing.admission_evidence.records.missing]
- GIVEN a harness report whose observation contains committed trace records but no admission decision event
- WHEN `molten test report validate` evaluates the report
- THEN validation fails closed with canonical failure evidence rather than treating the step as implicitly allowed

#### Scenario: Duplicate admission decisions fail validation
r[molten.testing.admission_evidence.records.duplicate]
- GIVEN a harness report whose observation contains two admission decision events
- WHEN the report is validated or gated as pass evidence
- THEN the validator rejects the report as malformed admission evidence

### Requirement: Admission requests are bound to suite steps
r[molten.testing.admission_evidence.step_binding] The validator MUST derive the expected admission request from the embedded suite step and MUST compare the recorded admission request against that derived request using canonical Preserves equality.

#### Scenario: Tampered admission request fails validation
r[molten.testing.admission_evidence.step_binding.tampered]
- GIVEN a suite step `<send "producer" "consumer" "hello">`
- WHEN the report records an admission request for a different actor, action, target, value, or effect metadata
- THEN validation rejects the report before accepting any committed trace or effect records for that step

### Requirement: Admission decisions are recomputed from embedded policy
r[molten.testing.admission_evidence.policy_recompute] The validator MUST recompute each admission decision from the embedded static policy fixture or policy refs and MUST reject reports whose recorded allow/deny decision or reason does not match the recomputed decision.

#### Scenario: Tampered deny-to-allow fails validation
r[molten.testing.admission_evidence.policy_recompute.deny_to_allow]
- GIVEN a suite policy that denies an assertion by a producer
- WHEN a report records that step as allowed
- THEN validation fails with an admission decision mismatch before considering the report pass evidence

#### Scenario: Stale policy fixture fails validation
r[molten.testing.admission_evidence.policy_recompute.stale_policy]
- GIVEN a report whose embedded policy fixture does not match the admission decisions recorded in its observations
- WHEN the report is validated or replayed
- THEN the validator rejects the stale report rather than trusting the previous runner output

### Requirement: Denied turns do not commit semantic actions
r[molten.testing.admission_evidence.deny_rollback] A denied non-effect turn MUST roll back before committing semantic runtime actions. Validation MUST reject a denied observation that contains committed message delivery, assertion commit, assertion retraction, observation side effects, storage mutation, adapter mutation, or any other committed action evidence after the denial.

#### Scenario: Denied assertion cannot commit
r[molten.testing.admission_evidence.deny_rollback.assertion]
- GIVEN a suite policy that denies `<assert "producer" "service.ready">`
- WHEN a report contains both a deny decision and an `<assertion-committed ...>` event for that step
- THEN validation rejects the report with denied-commit diagnostics

#### Scenario: Denied send cannot deliver
r[molten.testing.admission_evidence.deny_rollback.send]
- GIVEN a suite policy that denies a send from one actor to another
- WHEN a report contains both a deny decision and a message-delivered trace for that step
- THEN validation rejects the report and the pass-evidence gate refuses it

### Requirement: Denied effects are suppressed
r[molten.testing.admission_evidence.denied_effect_suppression] A denied effect step MUST NOT issue ambient effect request or effect response records. Validation MUST reject denied clock, random, storage, network, filesystem, process, blob, or adapter-effect steps that contain effect request/response evidence.

#### Scenario: Denied clock has no effect response
r[molten.testing.admission_evidence.denied_effect_suppression.clock]
- GIVEN a suite policy that denies a clock request
- WHEN a report records an effect request or response for that denied clock step
- THEN validation fails because the ambient observation crossed the effect boundary after denial

### Requirement: Admission divergence is first-class replay evidence
r[molten.testing.admission_evidence.policy_divergence] Deterministic replay MUST classify mismatches at the admission decision boundary as `policy-decision` divergence and MUST report the first divergent step before downstream trace, effect, output, or state-hash differences.

#### Scenario: Replay stops at changed admission decision
r[molten.testing.admission_evidence.policy_divergence.changed_decision]
- GIVEN a recorded report with an admission decision event
- WHEN replay recomputes a different request or decision for the same step
- THEN replay fails with `policy-decision` divergence at that step and includes expected/actual admission evidence refs or diagnostics

### Requirement: Gate receipts include admission checks
r[molten.testing.admission_evidence.gate_checks] Successful pass-evidence gate receipts MUST include explicit checks for admission policy schema/support, per-step admission decision validation, denied turn rollback, and denied effect suppression.

#### Scenario: Gate receipt lists admission checks
r[molten.testing.admission_evidence.gate_checks.receipt]
- GIVEN a deterministic harness report that validates and replays successfully
- WHEN `molten test gate check` emits a gate receipt
- THEN the receipt includes passed checks named `admission-policy`, `admission-decisions`, `deny-rollback`, and `denied-effect-suppression` in addition to schema, effect-log, budget, actor-registry, and deterministic-replay checks

### Requirement: Static policy fixtures remain declarative and replaceable
r[molten.testing.admission_evidence.static_policy_boundary] The initial static Preserves policy fixture MUST remain declarative, canonical, and side-effect free, and the validator MUST be structured so Nickel contracts, Basalt/UCAN authority context, and reviewed Steel predicates can augment or replace the fixture without removing fail-closed admission evidence validation.

#### Scenario: Dynamic predicate cannot bypass admission evidence
r[molten.testing.admission_evidence.static_policy_boundary.dynamic_predicate]
- GIVEN a future reviewed Steel predicate participates in an admission decision
- WHEN the decision is recorded in a harness report
- THEN the report still contains canonical admission request/decision evidence plus predicate receipt refs, and validation fails closed if either the decision or predicate evidence is missing

### Requirement: Policy preflight is required before side effects
r[molten.testing.policy_boundary.preflight_receipt] Evidence-bearing harness runs MUST perform static policy preflight before any runtime turn, semantic commit, or ambient effect request can execute. The run report MUST include canonical policy preflight evidence bound to the policy used for admission decisions.

#### Scenario: Report contains policy gate evidence
r[molten.testing.policy_boundary.preflight_receipt.report]
- GIVEN a deterministic harness suite with no explicit policy fixture or with a static deny-rule policy fixture
- WHEN the local harness executes the suite
- THEN the report contains a canonical `<policy-gate-v1 "molten.harness.policy-gate.v1" ...>` record before step observations are accepted as pass evidence

#### Scenario: Missing policy gate fails validation
r[molten.testing.policy_boundary.preflight_receipt.missing]
- GIVEN a harness report whose observations contain admission decisions but whose report lacks policy preflight evidence
- WHEN `molten test report validate` evaluates the report
- THEN validation fails closed before accepting the admission decisions as pass evidence

### Requirement: Policy snapshots are canonical and bound to suites
r[molten.testing.policy_boundary.policy_snapshot] The policy preflight gate MUST reference a canonical policy snapshot ref derived from the embedded suite policy. Omitted policies MUST normalize to an explicit allow-all policy snapshot, and explicit policies MUST normalize to canonical Preserves values whose refs are checked during report validation.

#### Scenario: Stale policy ref fails validation
r[molten.testing.policy_boundary.policy_snapshot.stale]
- GIVEN a report whose embedded suite policy or policy gate ref has been tampered after execution
- WHEN the report is validated or gated
- THEN validation rejects the report because the policy gate ref no longer matches the embedded policy snapshot

### Requirement: Nickel static boundary is explicit
r[molten.testing.policy_boundary.nickel_static] Static declarative policy/config/schema gates MUST be represented as Nickel-compatible static boundary evidence. Until the Nickel evaluator is fully integrated, the local harness MUST mark the current Preserves deny-rule fixture as a static Nickel-compatible subset rather than treating parser success as sufficient evidence.

#### Scenario: Static subset is identified
r[molten.testing.policy_boundary.nickel_static.marker]
- GIVEN a local harness report using the current Preserves deny-rule policy fixture
- WHEN a pass-evidence gate inspects the policy preflight evidence
- THEN the evidence identifies the static engine, Nickel-compatible contract marker, canonical policy snapshot ref, and static-boundary check result

### Requirement: Basalt policy gate evidence is explicit
r[molten.testing.policy_boundary.basalt_gate] Policy preflight decisions MUST be represented as Basalt gate evidence or a local harness Basalt-preflight marker until real Basalt/UCAN context refs are integrated. Missing, unsupported, or stale Basalt policy gate evidence MUST fail closed.

#### Scenario: Gate receipt lists Basalt policy check
r[molten.testing.policy_boundary.basalt_gate.receipt]
- GIVEN a deterministic report that validates, replays, and passes policy preflight
- WHEN `molten test gate check` emits a pass receipt
- THEN the receipt includes a `basalt-policy-gate` check and artifact refs for the policy snapshot and policy gate evidence

### Requirement: Steel predicates require reviewed callable receipts
r[molten.testing.policy_boundary.steel_review] Steel predicates, dynamic predicates, or trusted callables MUST NOT be accepted as ordinary static policy data. Any policy that references Steel/dynamic predicates MUST include reviewed callable receipt evidence before it can participate in admission; until that review path exists, local harness policy fixtures MUST reject such predicates fail-closed.

#### Scenario: Unreviewed Steel predicate is rejected
r[molten.testing.policy_boundary.steel_review.unreviewed]
- GIVEN a suite policy fixture containing an unreviewed `<steel-predicate ...>` or `<dynamic-predicate ...>` record
- WHEN the harness parses or preflights the suite
- THEN the suite is rejected before runtime execution and no side-effect-bearing report is produced

### Requirement: Gate receipts include policy boundary checks
r[molten.testing.policy_boundary.gate_receipts] Successful pass-evidence gate receipts MUST include checks for policy preflight, Nickel static policy boundary, Basalt policy gate evidence, and Steel predicate review in addition to admission-decision, deny-rollback, denied-effect-suppression, budget, actor-registry, effect-log, report-schema, and deterministic replay checks.

#### Scenario: Policy boundary checks are receipt evidence
r[molten.testing.policy_boundary.gate_receipts.checks]
- GIVEN a valid deterministic harness report
- WHEN the gate accepts it as pass evidence
- THEN the canonical gate receipt lists `policy-preflight`, `nickel-static-policy`, `basalt-policy-gate`, and `steel-predicate-review` as passed checks

### Requirement: Capability context evidence is canonical
r[molten.testing.capability_context.fixture_schema] Evidence-bearing harness suites MUST represent local authority inputs as canonical capability context evidence or refs. The first deterministic fixture MAY be a static Preserves `<capabilities-v1 "molten.harness.capabilities.v1" [...]>` record, but report and gate validation MUST treat it as capability evidence rather than ambient test setup.

#### Scenario: Suite declares local grants
r[molten.testing.capability_context.fixture_schema.grants]
- GIVEN a harness suite that grants `producer` authority to send to `consumer` and request logical time
- WHEN the suite is parsed for an evidence-bearing run
- THEN the capability fixture is canonicalized, hashed, and included in run identity and report evidence

#### Scenario: Missing capability fixture is explicit
r[molten.testing.capability_context.fixture_schema.default]
- GIVEN a harness suite with no explicit capability fixture
- WHEN the harness normalizes the suite for an evidence-bearing profile
- THEN the default capability context is explicit, deterministic, and validated rather than inferred from actor names or prior ambient state

### Requirement: Admission requests are bound to authority context
r[molten.testing.capability_context.request_binding] Each admission request MUST be authorized against the embedded capability context before side effects. Admission evidence MUST be bound to the capability context ref and to the matching grant or denial reason used for authorization.

#### Scenario: Send request uses matching grant
r[molten.testing.capability_context.request_binding.send]
- GIVEN a suite step `<send "producer" "consumer" "hello">`
- AND a capability context granting `producer` `send` authority to `consumer`
- WHEN the step is admitted
- THEN the admission evidence identifies the capability context and the decision is reproducible from the matching grant

#### Scenario: Tampered authority binding fails validation
r[molten.testing.capability_context.request_binding.tampered]
- GIVEN a report whose recorded admission decision references an authority context or grant not matching the embedded suite capability fixture
- WHEN the report is validated or gated
- THEN validation rejects the report before accepting any committed trace or effect records

### Requirement: Authorization denies by default
r[molten.testing.capability_context.deny_by_default] The harness runtime MUST deny admission when no matching capability grant or proof authorizes the request. Missing capability evidence MUST NOT be treated as implicit authority.

#### Scenario: Send without grant is denied
r[molten.testing.capability_context.deny_by_default.send]
- GIVEN a suite step where `producer` sends to `consumer`
- AND the capability context contains no matching send grant
- WHEN the step executes
- THEN the runtime records an admission denial, rolls the turn back, and emits no message-delivered evidence

#### Scenario: Observe without grant is denied
r[molten.testing.capability_context.deny_by_default.observe]
- GIVEN a suite step where an actor observes a dataspace pattern
- AND the capability context contains no matching observe grant
- WHEN the step executes
- THEN the runtime records an admission denial and does not register the observer

### Requirement: Ambient effects require capability grants
r[molten.testing.capability_context.effect_authority] Effect-producing steps, including clock, random, storage, network, filesystem, process, blob, or adapter effects, MUST require explicit capability grants or proof evidence before any effect request or response record is emitted.

#### Scenario: Clock without grant has no effect request
r[molten.testing.capability_context.effect_authority.clock]
- GIVEN a suite step `<clock "producer">`
- AND no matching clock capability grant
- WHEN the step executes
- THEN admission denies the request, rollback evidence is recorded, and no effect request or effect response appears in the observation

#### Scenario: Tampered denied effect response fails validation
r[molten.testing.capability_context.effect_authority.tampered]
- GIVEN a report with a denied clock step due to missing authority
- WHEN the report also contains an effect request or effect response for that step
- THEN report validation rejects the report as crossing the effect boundary after denial

### Requirement: Capability authorization composes with static policy
r[molten.testing.capability_context.policy_composition] Capability authorization and static policy admission MUST compose fail-closed. A step is allowed only when both the authority context authorizes the request and the static policy layer allows it. A denial from either layer MUST prevent side effects and be represented in canonical admission evidence.

#### Scenario: Granted but policy-denied assertion is denied
r[molten.testing.capability_context.policy_composition.policy_denies]
- GIVEN a capability context granting `producer` authority to assert `service.ready`
- AND a static policy denying that same assertion
- WHEN the assertion step executes
- THEN the final admission decision is deny and validation can recompute that denial from both embedded authority and policy evidence

#### Scenario: Policy allows but capability missing is denied
r[molten.testing.capability_context.policy_composition.capability_denies]
- GIVEN no static deny rule for a send step
- AND no matching send capability grant
- WHEN the send step executes
- THEN the final admission decision is deny because authority is missing

### Requirement: Capability evidence is recomputed during validation
r[molten.testing.capability_context.validation] Report validation MUST recompute capability authorization from the embedded capability context and MUST reject missing, malformed, stale, unsupported, or tampered capability evidence.

#### Scenario: Stale capability ref fails validation
r[molten.testing.capability_context.validation.stale]
- GIVEN a report whose embedded capability fixture changed after execution without updating recorded authority evidence
- WHEN the report is validated or accepted by a pass-evidence gate
- THEN validation fails closed because capability refs no longer bind to the embedded suite

#### Scenario: Tampered grant fails validation
r[molten.testing.capability_context.validation.tampered_grant]
- GIVEN a report that records an allowed send decision
- WHEN the embedded capability context is tampered to remove the matching send grant
- THEN validation rejects the report rather than trusting the recorded allow decision

### Requirement: Capability divergence is first-class replay evidence
r[molten.testing.capability_context.capability_divergence] Deterministic replay SHOULD classify mismatches at the authority boundary as `capability-decision` divergence, or MUST include equivalent authority-mismatch diagnostics before downstream trace, effect, output, or state-hash differences are reported.

#### Scenario: Replay stops at changed capability decision
r[molten.testing.capability_context.capability_divergence.changed]
- GIVEN a recorded report with an admission decision allowed by capability evidence
- WHEN replay recomputes a missing-grant denial for the same step
- THEN replay fails at the authority boundary with capability-decision or authority-mismatch diagnostics for the first divergent step

### Requirement: Gate receipts include capability checks
r[molten.testing.capability_context.gate_receipts] Successful pass-evidence gate receipts MUST include checks for capability context presence, grant/proof validation, deny-without-capability behavior, and authority-ref binding. Receipts MUST include artifact refs for the capability context and any future Basalt/UCAN proof bundle used for authorization.

#### Scenario: Gate receipt lists capability checks
r[molten.testing.capability_context.gate_receipts.checks]
- GIVEN a deterministic harness report that validates and replays successfully with capability evidence
- WHEN `molten test gate check` emits a pass receipt
- THEN the receipt includes passed checks named `capability-context`, `capability-grants`, `deny-without-capability`, and `authority-ref-binding`

### Requirement: Basalt/UCAN replacement seam is preserved
r[molten.testing.capability_context.basalt_ucan_path] The local static grant fixture MUST be structured so Basalt/UCAN proof refs, caveats, attenuation chains, revocation evidence, and authority receipts can replace or augment it without removing fail-closed capability validation.

#### Scenario: Future UCAN proof cannot bypass evidence
r[molten.testing.capability_context.basalt_ucan_path.future_ucan]
- GIVEN a future report whose admission decision depends on UCAN proof evidence
- WHEN the proof ref, caveat evidence, revocation check, or authority receipt is missing or stale
- THEN validation fails closed rather than treating the action as authorized
