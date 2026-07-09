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

### Requirement: Non-empty harness UCAN proofsets require verification receipts
r[molten.testing.capability_context.ucan_proofset_validation] Harness reports MAY include non-empty UCAN proofsets only when matching UCAN verification receipts are embedded or referenced, and validation MUST fail closed unless each proofset ref, token ref, proof ref, derived grant ref, caveat decision, revocation fact, replay fact, and request ref matches the embedded suite and observation evidence.

#### Scenario: Verified proofset is accepted
- GIVEN a harness suite whose capability gate contains a non-empty UCAN proofset and matching verification receipts
- WHEN report validation recomputes the capability gate
- THEN validation accepts the proofset only if every receipt and derived grant ref binds to the suite capability context and observed request.

#### Scenario: Stale proofset is rejected
- GIVEN a report whose UCAN verification receipt was produced for a different request, suite, holder, session, resource, ability, or proofset ref
- WHEN report validation runs
- THEN validation rejects the report before pass evidence can be emitted.

### Requirement: Harness admission binds Basalt enforcement receipts
r[molten.testing.capability_context.basalt_enforcement_receipts] Harness admission for Basalt-governed actions MUST call Basalt enforcement over the selected contract, requested resource, requested ability, and verified UCAN-derived grants, and MUST bind the Basalt enforcement receipt into observation, capability gate, and pass gate evidence.

#### Scenario: Basalt enforcement receipt admits action
- GIVEN UCAN verification derives a grant for the requested resource and ability
- AND the selected Basalt policy contract permits that resource and ability
- WHEN the harness admits the action
- THEN the observation carries an authority receipt that binds the UCAN verification receipt, derived grant refs, Basalt policy refs, Basalt enforcement receipt, and request ref.

#### Scenario: Mismatched Basalt policy denies action
- GIVEN UCAN verification derives a matching grant
- BUT the Basalt contract policy does not permit the requested resource or ability
- WHEN the harness attempts to admit the action
- THEN admission denies and the report contains deny evidence instead of committed side-effect evidence.

### Requirement: UCAN/Basalt fixtures cover positive and negative authority cases
r[molten.testing.capability_context.ucan_negative_fixtures] Harness tests SHOULD include positive UCAN/Basalt admission fixtures and negative fixtures for invalid signatures, unknown keys, wrong holder, wrong audience, wrong session, wrong context, wrong resource, wrong ability, expired or not-yet-valid tokens, revoked issuers or proofs, missing caveat evidence, replay denial, mismatched Basalt policy, local fixture fallback attempts, and tampered receipt refs.

#### Scenario: Negative fixture fails closed before pass gate
- GIVEN a UCAN/Basalt negative fixture with one invalid authority binding
- WHEN the harness runs, replays, validates, or gates the report
- THEN the fixture fails closed before emitting pass evidence
- AND diagnostics identify the invalid binding class.

### Requirement: NixOS multi-node VM topology
r[molten.testing.nixos_vm_multinode.topology] Molten MUST provide a NixOS VM integration test topology, implemented with `testers.runNixOSTest` or an equivalent NixOS test driver, that starts at least two Molten nodes with explicit VM networking, headless configuration, current flake/package inputs, isolated state roots, and no undeclared host state.

#### Scenario: VM topology starts two isolated Molten nodes
- GIVEN the current Molten source tree and Nix inputs
- WHEN the NixOS VM integration test topology is built
- THEN it defines at least two headless NixOS nodes with Molten installed from the same package derivation
- AND each node has an explicit state root, persistent identity location, and declared VM network identity.

### Requirement: Molten node service runs inside each VM
r[molten.testing.nixos_vm_multinode.node_service] Molten MUST run the real Molten node daemon or control loop under systemd inside each VM node, with startup, health, control-loop, shutdown, persistent-identity, and state-root evidence collected as canonical receipts.

#### Scenario: VM node readiness is receipt backed
- GIVEN a VM node configured for Molten
- WHEN the node service reaches ready state
- THEN the test collects startup and health receipt refs for the configured state root and persistent identity
- AND shutdown or restart collects matching node shutdown or recovery evidence.

### Requirement: Cross-node node-control workflow coverage
r[molten.testing.nixos_vm_multinode.control_workflow] Molten MUST exercise cross-node node-control workflow bundle handoff between VM nodes, including peer-ticket or endpoint evidence, authority evidence, bundle apply, reconcile, ack, and protocol-gate receipts.

#### Scenario: Bundle handoff crosses the VM network
- GIVEN two VM nodes with admitted peer and authority evidence
- WHEN `node-a` sends or stages a node-control workflow bundle for `node-b`
- THEN `node-b` applies or denies the bundle through the same control inbox and control-loop path used by the node daemon
- AND the final evidence binds apply, reconcile, ack, protocol-gate, ingress, queue, and control receipt refs.

### Requirement: Cross-node service, job, and coordination paths
r[molten.testing.nixos_vm_multinode.service_job_coordination] Molten SHOULD exercise at least one remote dataspace or service exchange, one job worker handoff or execution path, and one coordination operation across the VM nodes, binding each child receipt into the VM test run evidence.

#### Scenario: VM test binds distributed child receipts
- GIVEN a passing multi-node VM run
- WHEN the test run receipt is emitted
- THEN it includes child refs for a remote dataspace or service exchange, a job worker path, and a coordination operation
- AND each child receipt preserves its normal authority, policy, resource, provenance, source-gate, and retention checks separately.

### Requirement: Restart and durability VM scenario
r[molten.testing.nixos_vm_multinode.restart_durability] Molten MUST include a VM scenario that restarts or stops a node while control work is queued or partially dispatched, then verifies ledger readback, active-lock handling, queued request idempotency, and fail-closed recovery diagnostics.

#### Scenario: Restart handles queued control work deterministically
- GIVEN a control request is queued for a VM node
- WHEN the node is restarted before the request is fully dispatched
- THEN the resumed node either completes the request idempotently with matching receipt refs or emits a recovery denial before side effects
- AND the VM test evidence binds active-lock, inbox, outbox, ledger-readback, startup, shutdown, and recovery diagnostics.

### Requirement: Canonical NixOS VM test receipts
r[molten.testing.nixos_vm_multinode.receipts] Molten MUST emit canonical VM-level receipts for NixOS VM tests, including topology refs, node evidence refs, Nix input or store refs, scenario and fault-profile refs, child workflow refs, replay status, diagnostics, log refs, decision status, and explicit evidence-only caveats.

#### Scenario: Terminal output is not authoritative VM evidence
- GIVEN a VM integration test completes
- WHEN the result is evaluated by CI, release, or operator workflows
- THEN pass or deny status is read from canonical `nixos-vm-test-run-v1` or equivalent receipt evidence
- AND raw terminal output, QEMU logs, and systemd journals are bound as diagnostic refs rather than treated as authoritative pass evidence.

### Requirement: Explicit Nix/CI VM gate surface
r[molten.testing.nixos_vm_multinode.ci_gate] Molten SHOULD expose the multi-node VM test through an explicit Nix check or app with headless configuration and documented KVM/CI requirements. The gate MUST NOT silently convert skipped or unsupported VM execution into passing evidence.

#### Scenario: Missing VM support does not mint pass evidence
- GIVEN a CI environment without the required VM execution support
- WHEN the multi-node NixOS VM test check is requested
- THEN Molten emits a diagnostic failure, skip receipt, or unavailable status that is not accepted as pass evidence
- AND any default fast validation gate documents whether the VM test was executed or intentionally excluded.

### Requirement: Production-shaped multi-node soak workflow
r[molten.prod_soak.multi_node_live_workflow] Molten MUST provide a production-shaped multi-node soak workflow that exercises live peer tickets, node-control workflow bundles, remote dataspace or service exchange, job worker execution, coordination operations, and evidence export across at least two persistent node state roots.

#### Scenario: Multi-node soak binds child evidence
- GIVEN two or more nodes with persistent identities and admitted peer evidence
- WHEN the soak workflow completes
- THEN the soak receipt binds node startup refs, peer-ticket refs, node-control workflow refs, remote/service refs, job refs, coordination refs, and evidence-export refs for every participating node.

### Requirement: Network and transport fault matrix
r[molten.prod_soak.network_fault_matrix] Molten SHOULD test live or simulated network and transport faults including delay, drop, partition, rejoin, stale tickets, wrong authority grants, duplicate operations, conflicting operations, and corrupted or missing transport receipts.

#### Scenario: Stale ticket denies before side effects
- GIVEN a soak scenario with a stale or wrong live peer ticket
- WHEN a node-control request is sent or applied
- THEN Molten emits deny diagnostics before receiver-side control side effects are accepted.

### Requirement: Durability and restart soak
r[molten.prod_soak.durability_restart] Molten MUST include restart and durability scenarios covering queued control requests, active locks, ledger readback, chunk/artifact availability, retention state, and recovery receipts.

#### Scenario: Restart preserves queued request semantics
- GIVEN a node restarts while a control request is queued but not fully dispatched
- WHEN the soak harness resumes the node
- THEN the resulting receipts show deterministic idempotent handling of the queued request or a fail-closed recovery denial with diagnostics.

### Requirement: Soak replay and evidence boundary
r[molten.prod_soak.replay_and_evidence] Molten MUST emit canonical soak receipts that bind topology refs, fault-profile refs, child evidence refs, replay status, first-divergence diagnostics where applicable, and explicit non-replayable live caveats.

#### Scenario: Live-only observation is excluded from deterministic pass claim
- GIVEN a soak scenario includes an unrecorded live transport observation
- WHEN the soak receipt is evaluated for deterministic pass evidence
- THEN the observation is marked non-replayable and excluded or denied unless a recorded delivery log binds the event.

### Requirement: Performance and resource envelope
r[molten.prod_soak.performance_resource_envelope] Molten SHOULD track production-soak resource envelopes for queue depth, receipt growth, store growth, delivery latency, recovery time, resource pressure, and retained state growth, with explicit thresholds and diagnostics.

#### Scenario: Resource envelope breach is visible
- GIVEN a soak run exceeds a configured queue-depth or store-growth threshold
- WHEN the final soak receipt is emitted
- THEN it records degraded or deny status with the relevant resource measurements and child receipt refs.

### Requirement: Multi-node VM framed stream coverage
r[molten.testing.nixos_vm_multinode.framed_stream] Molten SHOULD extend the NixOS multi-node VM test to exercise at least one admitted framed Iroh bidirectional stream between VM nodes and bind the framed-stream receipts into the VM test-run evidence.

#### Scenario: VM test binds framed stream child receipt
- GIVEN two VM nodes with admitted peer, authority, policy, resource, and router registration evidence
- WHEN a canonical Preserves envelope crosses a framed Iroh stream between the nodes
- THEN the VM test-run receipt includes child refs for router admission, stream session, framed-envelope pass receipt, and downstream node-control or protocol-session admission
- AND the receipt states that live stream observations are non-replayable unless separately recorded.

#### Scenario: VM denial covers unsupported ALPN or malformed frame
- GIVEN a VM test attempts an unsupported ALPN connection or sends a malformed framed envelope
- WHEN the framed stream path evaluates the attempt
- THEN Molten emits deny evidence before state mutation
- AND the VM test binds the denial as diagnostic coverage rather than transport-derived authority.

### Requirement: Multi-node VM network diagnostics evidence
r[molten.testing.nixos_vm_multinode.network_diagnostics] Molten SHOULD bind local network diagnostics reports, connectivity probe receipts, route/interface watcher snapshots, and metrics snapshot refs into the NixOS multi-node VM test-run evidence when the host environment can execute those checks.

#### Scenario: VM run binds diagnostics child refs
- GIVEN a multi-node VM test completes network diagnostics and metrics snapshots for each node
- WHEN the VM test-run receipt is emitted
- THEN it includes child refs for diagnostics reports, connectivity probes, watcher snapshots, and metrics snapshots
- AND raw terminal logs remain diagnostic refs rather than authoritative pass evidence.

#### Scenario: Missing host support does not mint diagnostic pass evidence
- GIVEN the host cannot perform a required VM network diagnostic or port-map probe
- WHEN the VM check requests that diagnostic
- THEN Molten records unavailable or degraded diagnostics
- AND the VM check does not convert the unavailable diagnostic into pass evidence.

### Requirement: Deterministic drift comparison core
r[molten.testing.deterministic_drift.comparison_core] Molten MUST provide a pure deterministic drift comparator that accepts paired workflow evidence summaries, canonical receipt or report refs, and explicit allowed-variance declarations, then returns a pass or deny result with first-drift diagnostics.

#### Scenario: Equal canonical evidence passes drift comparison
- GIVEN two evidence summaries produced from the same declared deterministic inputs
- WHEN the drift comparator evaluates their canonical refs and normalized values
- THEN comparison passes only if all semantic refs and normalized canonical values match.

#### Scenario: Unexplained ref drift fails closed
- GIVEN two evidence summaries from the same declared inputs with different report or receipt refs
- WHEN no allowed-variance declaration accounts for the difference
- THEN comparison fails closed with a diagnostic naming the first differing ref or field.

### Requirement: Allowed variance is explicit and canonical
r[molten.testing.deterministic_drift.variance_declarations] Deterministic drift checks MUST allow volatile fields only when each variance is explicitly declared, justified by a reason class, and removed or normalized through canonical comparison rules before equality is evaluated.

#### Scenario: Declared volatile field is normalized
- GIVEN two workflow evidence summaries that differ only in a declared non-semantic volatile field
- WHEN the drift comparator applies the allowed-variance declaration
- THEN the normalized semantic evidence matches and the comparison may pass.

#### Scenario: Undeclared volatile field fails comparison
- GIVEN two workflow evidence summaries that differ in an undeclared field
- WHEN drift comparison runs
- THEN the comparison fails closed even if the field appears incidental in rendered logs.

### Requirement: Fresh rerun drift gate
r[molten.testing.deterministic_drift.fresh_rerun_gate] Molten MUST provide an explicit drift gate that runs selected evidence-bearing workflows in fresh isolated state roots with the same declared inputs and compares their canonical evidence through the drift comparator.

#### Scenario: Same workflow rerun produces same evidence
- GIVEN a deterministic evidence-bearing workflow and a declared input set
- WHEN the drift gate runs the workflow in separate fresh state roots
- THEN the gate compares canonical output refs from each run and passes only if semantic evidence is identical after declared normalization.

#### Scenario: Ambient state drift is denied
- GIVEN a workflow that reads undeclared ambient state and changes canonical evidence between fresh runs
- WHEN the drift gate compares the outputs
- THEN the gate fails closed with an ambient-state or unexplained-drift diagnostic.

### Requirement: Release workflows are covered by drift checks
r[molten.testing.deterministic_drift.release_workflows] Molten SHOULD cover dogfood local-node, sealed repro verify/unpack, release bundle verify, release promotion, release export verification, and deterministic VM child evidence with drift checks where those workflows claim deterministic evidence.

#### Scenario: Dogfood evidence is stable across fresh roots
- GIVEN the same source tree and declared dogfood inputs
- WHEN the drift gate runs dogfood local-node twice in fresh state roots
- THEN release-gate, replay-verify, replay-index, bundle-verify, promotion, and export-verify semantic evidence refs match or fail with declared variance diagnostics.

### Requirement: Drift gate has positive and negative fixtures
r[molten.testing.deterministic_drift.negative_fixtures] Molten SHOULD test deterministic drift validation with positive same-input/same-ref fixtures and negative fixtures for injected ref drift, undeclared volatile fields, ambient state use, unstable map ordering, and rendered-output-only changes.

#### Scenario: Injected drift fixture is rejected
- GIVEN a fixture pair whose second evidence summary has a changed canonical child ref
- WHEN the drift comparator evaluates the pair
- THEN validation fails closed with a first-drift diagnostic before accepting the workflow as deterministic evidence.

### Requirement: Drift gate has an explicit validation surface
r[molten.testing.deterministic_drift.gate_surface] Molten SHOULD expose deterministic drift validation through an explicit Nix check, app, or release-readiness command. The gate MUST NOT treat retry success as proof that drift was absent.

#### Scenario: Retry does not mask drift
- GIVEN a workflow that alternates between two canonical evidence refs across runs
- WHEN the drift validation surface is invoked
- THEN the gate reports drift instead of retrying until two matching outputs appear.

### Requirement: Drift workflow is documented
r[molten.testing.deterministic_drift.docs] User-facing documentation SHOULD describe which workflows are compared, what refs are authoritative, how allowed variance is declared, and how to diagnose first-drift failures.

#### Scenario: Operator diagnoses a drift failure
- GIVEN a drift gate failure in release evidence review
- WHEN an operator follows the documented workflow
- THEN they can identify the first differing canonical ref, the workflow step that emitted it, and whether a variance declaration or code fix is required.

### Requirement: Requirement coverage manifest
r[molten.testing.requirement_traceability.manifest] Molten MUST be able to generate a deterministic requirement coverage manifest that lists accepted and changed `r[...]` requirement ids, their source spec locations, positive verification evidence, negative verification evidence, validation commands, evidence artifact refs, and exemption status.

#### Scenario: Manifest records positive and negative coverage
- GIVEN accepted testing and evidence requirements with associated verification markers
- WHEN the requirement coverage manifest is generated
- THEN each covered requirement entry identifies its requirement id, source spec, positive test or evidence, negative test or evidence, validation command, and current coverage status.

#### Scenario: Documentation-only requirement is explicitly exempted
- GIVEN a requirement whose only required outcome is operator documentation
- WHEN the manifest is generated
- THEN the entry records a reviewed exemption class and supporting documentation evidence instead of silently appearing covered by unrelated tests.

### Requirement: Traceability gate requires covered evidence-bearing requirements
r[molten.testing.requirement_traceability.coverage_gate] Molten MUST provide a traceability gate that fails closed, or emits non-pass evidence, when an evidence-bearing or changed requirement lacks required positive and negative coverage and has no documented exemption.

#### Scenario: Missing negative coverage fails the gate
- GIVEN a changed evidence-bearing requirement with a positive test and no negative test or exemption
- WHEN the traceability gate runs
- THEN the gate fails closed with a diagnostic naming the requirement id and missing negative coverage.

#### Scenario: Complete coverage passes the gate
- GIVEN a changed evidence-bearing requirement with positive coverage, negative coverage, validation command evidence, and no stale refs
- WHEN the traceability gate runs
- THEN the gate emits pass evidence for that requirement coverage entry.

### Requirement: Traceability detects stale references
r[molten.testing.requirement_traceability.stale_detection] Traceability validation MUST detect stale requirement ids, missing test targets, missing validation commands, missing evidence artifacts, and references to deleted or renamed specs.

#### Scenario: Stale test reference fails closed
- GIVEN a manifest entry that points to a test target or fixture path that no longer exists
- WHEN traceability validation runs
- THEN validation fails closed with a stale-reference diagnostic.

#### Scenario: Removed requirement id is not counted as covered
- GIVEN a coverage entry for a requirement id that no longer appears in accepted specs or active change deltas
- WHEN traceability validation runs
- THEN the entry is reported as stale and cannot satisfy coverage for any current requirement.

### Requirement: Traceability fixtures cover success and failure
r[molten.testing.requirement_traceability.fixtures] Molten SHOULD test traceability validation with fixtures for complete coverage, missing positive coverage, missing negative coverage, stale requirement ids, missing test targets, missing evidence artifact refs, and documented exemptions.

#### Scenario: Missing evidence fixture is denied
- GIVEN a traceability fixture with a requirement entry whose evidence artifact ref is absent
- WHEN fixture validation runs
- THEN the validator reports a denial for the missing evidence artifact ref.

### Requirement: Traceability has an explicit gate surface
r[molten.testing.requirement_traceability.nix_surface] Molten SHOULD expose requirement traceability validation through an explicit Nix or Cairn command that can be invoked by release evidence review and local development.

#### Scenario: Release review invokes traceability gate
- GIVEN a release candidate source tree
- WHEN release evidence validation requests requirement traceability
- THEN the explicit gate command emits a machine-readable result and a compact summary without requiring manual source search.

### Requirement: Traceability summary is operator-readable
r[molten.testing.requirement_traceability.operator_summary] Molten SHOULD render a compact traceability summary grouped by covered, exempt, missing-positive, missing-negative, stale-reference, and unsupported requirement entries.

#### Scenario: Summary names actionable gaps
- GIVEN a manifest with missing negative coverage and stale references
- WHEN the operator summary is rendered
- THEN it names the affected requirement ids, gap class, and next validation evidence needed.

### Requirement: Traceability workflow is documented
r[molten.testing.requirement_traceability.docs] User-facing documentation SHOULD explain how to add positive coverage, negative coverage, validation commands, evidence refs, and exemptions when adding or changing requirements.

#### Scenario: Contributor updates coverage with a requirement
- GIVEN a contributor adding a new evidence-bearing requirement
- WHEN they follow the traceability documentation
- THEN they add both positive and negative coverage entries or a reviewed exemption before the traceability gate can pass.

### Requirement: VM evidence is semantically validated
r[molten.testing.vm_evidence.semantic_validation] Molten MUST validate NixOS VM evidence by parsing canonical receipt contents, not only by checking marker strings or command success. Validation MUST bind the expected topology, node ids, state roots, Nix store refs, child workflow refs, replay status, diagnostics, decision status, and evidence-only caveats.

#### Scenario: Passing VM evidence validates by content
- GIVEN a completed multi-node VM test with topology, node evidence, VM test-run, and production-soak receipts
- WHEN the VM evidence validator evaluates the canonical receipts against the expected topology
- THEN validation passes only if receipt contents bind the expected nodes, package refs, state roots, child receipt refs, replay status, diagnostics, and pass decision
- AND raw terminal output is not accepted as a substitute for the canonical receipts.

#### Scenario: Marker-only evidence is rejected
- GIVEN VM-local files that contain expected receipt kind strings but omit required topology, child refs, replay status, or decision fields
- WHEN the VM evidence validator evaluates the files
- THEN validation fails closed with diagnostics for the missing semantic bindings.

### Requirement: VM check outputs preserve canonical evidence
r[molten.testing.vm_evidence.artifact_preservation] Molten MUST preserve canonical VM evidence receipts from platform integration checks as explicit Nix output artifacts with a manifest that binds artifact paths, receipt kinds, BLAKE3 content refs, diagnostic log refs, and evidence-only caveats.

#### Scenario: VM check output contains reviewable evidence
- GIVEN a successful `nixos-vm-multinode` check
- WHEN an operator inspects the realized Nix output path
- THEN the output contains a manifest plus the canonical topology, node evidence, VM test-run, production-soak, and child evidence receipts needed for review
- AND each manifest entry binds a stable content ref and receipt kind.

#### Scenario: Empty VM output cannot satisfy release evidence
- GIVEN a VM test derivation that completes but does not preserve canonical evidence artifacts
- WHEN release evidence validation evaluates the derivation output
- THEN the output is denied or marked unavailable for release-evidence purposes even if the build log contains passing assertions.

### Requirement: VM logs remain diagnostic evidence
r[molten.testing.vm_evidence.log_boundary] VM terminal output, QEMU logs, systemd journals, and rendered summaries MUST be treated as diagnostic evidence only. They MAY be preserved and referenced by the VM evidence manifest, but they MUST NOT replace canonical receipt validation for pass evidence.

#### Scenario: Log text cannot override a deny receipt
- GIVEN preserved VM logs that contain successful-looking text and a canonical VM test-run receipt with a deny decision
- WHEN VM evidence validation runs
- THEN validation follows the canonical deny receipt
- AND the successful-looking log text remains diagnostic-only.

### Requirement: VM semantic validation has negative fixtures
r[molten.testing.vm_evidence.negative_fixtures] Molten SHOULD test VM evidence validation with negative fixtures covering missing receipts, stale refs, tampered receipt bytes, wrong topology membership, wrong decision status, incomplete child refs, missing replay status, and unbound diagnostic logs.

#### Scenario: Tampered VM evidence fails closed
- GIVEN a previously passing VM evidence bundle whose node evidence or child receipt ref has been changed
- WHEN the semantic validator evaluates the bundle
- THEN validation fails closed before the bundle can satisfy release or pilot evidence review.

### Requirement: VM evidence inspection is documented
r[molten.testing.vm_evidence.docs] User-facing documentation SHOULD explain which VM output artifacts are authoritative, how to inspect the manifest and canonical receipts, and why logs are diagnostic-only.

#### Scenario: Operator follows VM evidence docs
- GIVEN an operator reviewing a realized VM check output
- WHEN they follow the documented inspection procedure
- THEN they can identify the authoritative VM receipts, their content refs, the validation decision, child workflow evidence, and diagnostic log refs without relying on raw build-log scraping.

### Requirement: Proof obligation manifests
r[molten.testing.proof_obligations.manifest] Molten SHOULD represent broad proof claims as deterministic proof-obligation manifests that list child obligations, subject refs, prerequisite refs, receipt refs, decisions, diagnostics, and evidence-only caveats.

#### Scenario: Aggregate proof lists child obligations
- GIVEN a workflow proof that depends on multiple semantic checks
- WHEN Molten renders the aggregate proof manifest
- THEN the manifest names each child obligation and the canonical receipt refs that satisfy it.

### Requirement: Standard proof obligation classes
r[molten.testing.proof_obligations.classes] Proof obligation manifests SHOULD distinguish input-validation, canonicalization, admission, mutation-boundary, replay-determinism, and fail-closed-negative obligations when those classes are part of a workflow claim.

#### Scenario: Mutation boundary is separate from admission
- GIVEN a workflow that denies an operation before mutation
- WHEN its proof manifest is rendered
- THEN admission evidence and no-mutation evidence appear as separate obligations.

### Requirement: Aggregate obligation gate
r[molten.testing.proof_obligations.aggregate_gate] Aggregate proof validation MUST fail closed when a required child obligation is missing, duplicated, bound to the wrong subject, bound to the wrong prerequisite, or has the wrong expected decision.

#### Scenario: Missing replay obligation denies aggregate proof
- GIVEN an aggregate proof requiring replay-determinism evidence
- WHEN the replay obligation receipt is absent
- THEN aggregate validation emits deny evidence for the missing child obligation.

### Requirement: Traceability can reference aggregate proofs
r[molten.testing.proof_obligations.traceability] Traceability MAY accept aggregate proof manifest refs as coverage evidence when the manifest exposes matching requirement ids and positive or negative coverage kinds.

#### Scenario: Requirement coverage comes from child obligation
- GIVEN an aggregate proof manifest with child obligations linked to requirement ids
- WHEN traceability consumes the manifest
- THEN the requirement is covered only by matching child obligations, not by the aggregate label alone.

### Requirement: Obligation summaries are operator-readable
r[molten.testing.proof_obligations.operator_summary] Proof obligation readbacks SHOULD group obligations by class, decision, subject, and missing or stale diagnostics.

#### Scenario: Summary names missing child
- GIVEN an aggregate proof manifest missing a mutation-boundary child
- WHEN the operator summary is rendered
- THEN it names the missing obligation class and subject ref.

### Requirement: Obligation Hegel properties
r[molten.testing.proof_obligations.hegel_properties] Proof obligation validation SHOULD include Hegel RS property tests for deterministic ordering, stable refs, missing-child denial, duplicate-child denial, mismatched-subject denial, and positive/negative substitution denial.

#### Scenario: Generated duplicate child denies
- GIVEN Hegel RS generates an aggregate manifest with duplicate child obligation ids
- WHEN aggregate validation runs
- THEN validation denies the aggregate proof.

### Requirement: Obligation fixtures
r[molten.testing.proof_obligations.fixtures] Proof obligation tests SHOULD include complete positive fixtures and negative fixtures for missing child, duplicate child, wrong subject, wrong prerequisite, stale receipt, and wrong expected decision.

#### Scenario: Wrong subject fixture fails
- GIVEN a child obligation receipt for a different subject ref
- WHEN aggregate validation runs
- THEN the aggregate proof is denied before satisfying coverage.

### Requirement: Obligation decomposition docs
r[molten.testing.proof_obligations.docs] Proof workflow documentation SHOULD explain how to decompose broad claims into child obligations and how aggregate proof manifests remain evidence-only.

#### Scenario: Contributor decomposes workflow claim
- GIVEN a contributor adds a new workflow proof
- WHEN they follow the documentation
- THEN they identify child obligations and attach explicit positive and negative receipts for review.

### Requirement: Verification run receipts
r[molten.testing.verification_run_receipts.schema] Molten MUST emit canonical verification run receipts for test, validation, and proof commands that are used as requirement coverage evidence.

#### Scenario: Command run emits a receipt
- GIVEN a proof command selected for requirement coverage
- WHEN the command completes
- THEN Molten emits a `verification-run-receipt-v1` artifact
- AND the artifact binds the requirement id, coverage kind, target, normalized argv, execution profile, exit status, captured output refs, and produced artifact refs.

### Requirement: Verification receipts bind command identity
r[molten.testing.verification_run_receipts.command_binding] Verification run receipts MUST bind command identity and execution profile without treating rendered logs as normative evidence.

#### Scenario: Changed command does not satisfy old coverage
- GIVEN a traceability entry that expects one normalized argv and execution profile
- WHEN a verification receipt names different argv or profile refs
- THEN traceability reports stale or mismatched evidence before accepting coverage.

### Requirement: Verification receipts bind artifacts
r[molten.testing.verification_run_receipts.artifact_binding] Verification run receipts MUST bind produced artifact refs and fail closed when a named artifact ref is missing, malformed, stale, or inconsistent with the command result.

#### Scenario: Tampered artifact ref denies coverage
- GIVEN a verification receipt whose produced artifact ref does not validate
- WHEN traceability consumes the receipt
- THEN the corresponding coverage entry is denied with an artifact-binding diagnostic.

### Requirement: Traceability accepts receipt-backed coverage
r[molten.testing.verification_run_receipts.traceability] Traceability SHOULD accept verification-run receipt refs as first-class positive and negative coverage inputs and SHOULD prefer them over raw command strings when both are present.

#### Scenario: Receipt-backed positive and negative coverage passes
- GIVEN a changed evidence-bearing requirement
- AND matching positive and negative verification-run receipts
- WHEN traceability scanning runs
- THEN the requirement is covered without relying on manually entered command text.

### Requirement: Compatibility coverage remains explicit
r[molten.testing.verification_run_receipts.compatibility] Compatibility coverage strings MAY remain supported, but traceability MUST identify whether each coverage entry is receipt-backed or compatibility-only.

#### Scenario: Compatibility entry is visible
- GIVEN a coverage entry supplied as raw requirement, kind, target, command, and ref fields
- WHEN the traceability summary is rendered
- THEN the entry remains usable under compatibility policy
- AND the summary identifies that no verification-run receipt backed the entry.

### Requirement: Verification receipt Hegel properties
r[molten.testing.verification_run_receipts.hegel_properties] Verification receipt validation SHOULD include Hegel RS property tests for stable canonical refs, command binding drift, requirement/kind mismatches, stale targets, malformed refs, and deny receipts not satisfying positive coverage.

#### Scenario: Generated receipt drift is denied
- GIVEN Hegel RS generates a valid receipt input and a mutated command or artifact binding
- WHEN both receipts are validated for the same traceability entry
- THEN only the unmutated matching receipt can satisfy coverage.

### Requirement: Verification receipt fixtures
r[molten.testing.verification_run_receipts.fixtures] Verification receipt coverage SHOULD include positive fixtures for matching pass and expected-deny receipts and negative fixtures for stale target, missing output, malformed artifact ref, wrong requirement id, and wrong coverage kind.

#### Scenario: Wrong coverage kind fails
- GIVEN a positive coverage slot and a verification-run receipt marked as negative coverage
- WHEN traceability validates the entry
- THEN the entry is denied as a kind mismatch.

### Requirement: Verification receipt workflow docs
r[molten.testing.verification_run_receipts.docs] User-facing proof workflow documentation SHOULD describe how to generate verification-run receipts and feed them into traceability.

#### Scenario: Contributor adds receipt-backed coverage
- GIVEN a contributor adds or changes a requirement
- WHEN they follow the proof workflow documentation
- THEN they generate positive and negative verification-run receipts and pass those refs to the traceability gate.

### Requirement: Receipt-backed coverage source model
r[molten.testing.receipt_driven_traceability.source_model] Traceability SHOULD accept canonical proof receipt refs as coverage sources and treat hand-authored coverage tuples as compatibility-only input.

#### Scenario: Receipt ref is a coverage source
- GIVEN a verification or proof receipt that names a requirement id and coverage kind
- WHEN traceability scanning receives the receipt ref
- THEN the scanner derives a coverage entry from the canonical receipt fields.

### Requirement: Coverage is derived from receipts
r[molten.testing.receipt_driven_traceability.coverage_derivation] Receipt-driven traceability MUST derive requirement id, coverage kind, target, command identity, artifact refs, and diagnostics from validated canonical receipt fields rather than from rendered logs.

#### Scenario: Derived entry binds artifact refs
- GIVEN a receipt with produced artifact refs
- WHEN coverage derivation runs
- THEN the derived traceability entry names those refs and validates their content-ref shape.

### Requirement: Raw coverage claims are labeled
r[molten.testing.receipt_driven_traceability.raw_claim_policy] Traceability summaries MUST identify compatibility-only raw coverage entries and MAY allow release profiles to require receipt-backed coverage for changed evidence-bearing requirements.

#### Scenario: Raw tuple remains visible
- GIVEN a raw coverage tuple without a receipt ref
- WHEN the summary is rendered
- THEN the entry is labeled compatibility-only rather than indistinguishable from receipt-backed evidence.

### Requirement: Stale receipt coverage denies
r[molten.testing.receipt_driven_traceability.stale_receipt_denial] Receipt-driven traceability MUST deny stale receipt refs, duplicate coverage receipts for the same slot unless policy permits aggregation, wrong requirement ids, wrong coverage kinds, malformed refs, and receipts whose decision cannot satisfy the requested coverage kind.

#### Scenario: Wrong requirement receipt fails
- GIVEN a coverage slot for one requirement
- AND a receipt naming another requirement id
- WHEN traceability derives coverage
- THEN the slot remains uncovered and the receipt is reported as stale or mismatched.

### Requirement: Receipt-driven traceability has a gate surface
r[molten.testing.receipt_driven_traceability.nix_gate] Molten SHOULD expose receipt-driven traceability through the same release, Nix, or Cairn gate surface used by existing traceability scanning.

#### Scenario: Release gate requires receipt-backed coverage
- GIVEN a release profile that requires receipt-backed traceability
- WHEN a changed evidence-bearing requirement has only raw tuple coverage
- THEN the gate denies or marks the requirement as not release-covered.

### Requirement: Receipt-driven Hegel properties
r[molten.testing.receipt_driven_traceability.hegel_properties] Receipt-driven traceability SHOULD include Hegel RS property tests for deterministic derivation, positive/negative separation, duplicate handling, stale receipt denial, and deny-monotonicity when bad receipts are added.

#### Scenario: Adding stale receipt cannot create pass
- GIVEN Hegel RS generates a passing receipt-backed coverage set
- WHEN a stale receipt for a deleted requirement is added
- THEN the resulting traceability decision is deny or the stale receipt is explicitly excluded by policy with diagnostics.

### Requirement: Receipt-driven coverage docs
r[molten.testing.receipt_driven_traceability.docs] User-facing documentation SHOULD explain how to provide receipt refs to traceability and how compatibility-only raw tuples differ from receipt-backed coverage.

#### Scenario: Contributor migrates raw coverage
- GIVEN an existing raw coverage tuple
- WHEN a contributor follows the documentation
- THEN they can replace it with a receipt ref that derives the same requirement coverage.

### Requirement: State-machine proof traces have a bounded contract
r[molten.testing.state_machine_proof.trace_contract] Molten MUST define a bounded proof trace step contract for state-machine evidence that binds before-state refs, transition or command refs, after-state refs, predicate or check names, decisions, diagnostics, and receipt refs.

#### Scenario: Trace step binds state and receipt evidence
- GIVEN a state-machine proof trace step
- WHEN Molten validates the step contract
- THEN the step identifies the prior state ref, transition or command ref, resulting state ref, decision, diagnostics, and receipt ref
- AND the step remains bounded for deterministic replay.

### Requirement: State-machine proof traces replay validate
r[molten.testing.state_machine_proof.trace_validator] Molten MUST validate state-machine proof traces by checking each step's receipt bindings and by ensuring adjacent steps chain through matching state refs.

#### Scenario: Valid proof trace replays
- GIVEN a proof trace whose steps have valid receipt refs and matching adjacent state refs
- WHEN Molten replay-validates the trace
- THEN validation passes
- AND the summary binds the accepted step count and final state ref.

### Requirement: State-machine proof trace validation fails closed
r[molten.testing.state_machine_proof.trace_validator_negative] Molten MUST reject state-machine proof traces with missing receipts, tampered diagnostics, stale before-state refs, wrong after-state refs, or out-of-order steps.

#### Scenario: Tampered trace denies
- GIVEN a proof trace whose receipt diagnostics or adjacent state refs have been modified
- WHEN Molten replay-validates the trace
- THEN validation denies the trace
- AND diagnostics identify the first invalid proof binding.

### Requirement: Distributed simulation fault plans are canonical
r[molten.testing.distributed_simulation.fault_plan_schema] Molten MUST define canonical distributed simulation records for topology, deterministic seed, scheduler profile, and fault plan. Fault plans MUST bind delay, drop, duplicate, reorder, partition, rejoin, crash, restart, and resource-pressure events by explicit peer, channel, operation, or time-window identifiers rather than ambient runtime state.

#### Scenario: Fault plan identity is stable
- GIVEN the same simulated topology, scheduler profile, seed, and ordered fault events
- WHEN Molten canonicalizes the simulation input
- THEN the resulting fault-plan ref is stable
- AND changing any peer, operation, event, or schedule field changes the canonical ref.

### Requirement: Distributed simulation core is deterministic
r[molten.testing.distributed_simulation.simulator_core] Molten MUST provide a pure deterministic distributed simulation core that evaluates explicit topology state, virtual time, queued messages, workflow commands, and fault events without reading clocks, files, networks, process state, environment variables, or ambient randomness.

#### Scenario: Same seed produces same simulated evidence
- GIVEN identical topology, seed, scheduler profile, workflow commands, and fault plan
- WHEN the simulator runs twice in fresh process state
- THEN both runs emit the same semantic event refs, final state refs, decisions, and diagnostics.

#### Scenario: Ambient state cannot affect simulation
- GIVEN a simulation input with no declared host or environment fields
- WHEN host paths, wall-clock time, process ids, or network availability differ
- THEN the simulator output remains unchanged or denies because an explicit required input is missing.

### Requirement: Distributed simulation emits run receipts
r[molten.testing.distributed_simulation.run_receipts] Molten MUST emit canonical `distributed-test-run-v1` or equivalent receipts that bind source or test binary refs, topology ref, seed ref, scheduler profile ref, fault-plan ref, child workflow refs, emitted event refs, final state refs, replay status, allowed variance declarations, diagnostics, and pass or deny decision.

#### Scenario: Run receipt explains a deny decision
- GIVEN a simulated stale-ticket, missing-authority, duplicate-operation, or partitioned workflow denial
- WHEN the simulation run receipt is emitted
- THEN the receipt identifies the first denied invariant, relevant child refs, and the fault-plan event that exposed the denial.

### Requirement: Distributed invariants have model coverage
r[molten.testing.distributed_simulation.property_invariants] Molten SHOULD cover distributed safety invariants with property or model tests, including operation-id idempotency, no authority from transport evidence, duplicate or reordered messages not advancing state twice, deny-before-side-effects, and restart replay preserving canonical refs.

#### Scenario: Duplicate delivery does not double commit
- GIVEN a generated workflow with a duplicate delivery fault for the same operation id
- WHEN the model test evaluates committed state transitions
- THEN at most one semantic commit is accepted for that operation id
- AND any replayed duplicate is represented by explicit idempotency evidence.

### Requirement: Distributed simulation fixtures cover positive and negative paths
r[molten.testing.distributed_simulation.fixtures] Molten SHOULD provide positive fixtures for admitted workflows under bounded benign faults and negative fixtures for stale evidence, unauthorized transport-derived trust, corrupted receipts, undeclared ambient state, and invariant violations.

#### Scenario: Unauthorized transport evidence denies
- GIVEN a simulated message with live or transport identity evidence but no matching authority, policy, or resource evidence
- WHEN the workflow attempts a privileged state transition
- THEN simulation emits a deny decision before side effects
- AND diagnostics state that transport evidence does not grant authority.

### Requirement: Distributed simulation docs explain evidence scope
r[molten.testing.distributed_simulation.docs] User-facing documentation SHOULD explain how distributed simulation evidence complements unit, CLI, VM, and live soak evidence, and MUST state that simulation receipts do not grant authority, policy, provenance, resource, source-gate, retention, transport, destructive-operation, or deployment trust.

#### Scenario: Reviewer distinguishes simulation from VM evidence
- GIVEN a reviewer inspects distributed simulation output
- WHEN they follow the documentation
- THEN they can identify the topology, seed, fault plan, canonical run receipt, covered invariants, and claims that remain reserved for VM or live pilot evidence.

### Requirement: Distributed CI profiles are explicit
r[molten.testing.distributed_ci.profile_matrix] Molten MUST define an explicit distributed test risk/cost matrix covering fast core checks, deterministic protocol simulation, CLI receipt workflows, VM smoke checks, executable VM fault checks, and soak or pilot evidence. Each matrix entry MUST name its command surface, expected artifact kinds, evidence scope, cost class, and release-review status.

#### Scenario: Release reviewer sees distributed test scope
- GIVEN the distributed CI matrix is rendered for a candidate tree
- WHEN a reviewer inspects the matrix
- THEN each profile identifies the command to run, authoritative receipt artifacts, unsupported or unavailable states, and claims that remain out of scope.

### Requirement: Distributed test metadata is bound canonically
r[molten.testing.distributed_ci.metadata_binding] Distributed test run evidence MUST bind source or tree refs, Nix input refs where applicable, test binary or package refs, profile and shard refs, seed refs, topology refs, fault-plan refs, emitted receipt refs, allowed variance declarations, and diagnostic log refs.

#### Scenario: Shard evidence is reproducible
- GIVEN a distributed test shard emits pass evidence
- WHEN the shard metadata is parsed
- THEN it identifies the source, Nix inputs, binary, profile, seed, topology, fault plan, child receipts, and declared variance needed to reproduce or audit the shard.

### Requirement: Traceability is required for distributed evidence gates
r[molten.testing.distributed_ci.traceability_required_gate] Release or CI review for distributed evidence-bearing requirements MUST require traceability coverage that includes positive evidence, negative evidence, validation commands, and artifact refs, or an explicit documented exemption. Missing, stale, or unsupported coverage MUST deny the distributed evidence gate.

#### Scenario: Missing negative coverage denies release evidence
- GIVEN a distributed requirement has positive VM evidence but no negative denial or exemption evidence
- WHEN the distributed CI traceability gate runs
- THEN the gate denies or marks the requirement incomplete before release evidence can pass.

### Requirement: Retry success is not pass evidence
r[molten.testing.distributed_ci.retry_policy] Distributed CI and release profiles that emit pass evidence MUST run with zero retries or otherwise bind every attempted run and deny retry-only success as proof of deterministic behavior. Exploratory reruns MAY produce diagnostic or quarantine evidence but MUST NOT satisfy pass gates without explicit review evidence.

#### Scenario: Flaky test passes only after retry
- GIVEN a distributed test fails on the first attempt and passes on a retry
- WHEN release evidence evaluates the run
- THEN the run is not accepted as deterministic pass evidence
- AND diagnostics identify the failed attempt and retry boundary.

### Requirement: Unsupported distributed profiles are unavailable, not passing
r[molten.testing.distributed_ci.unavailable_handling] Distributed CI profiles requiring VM, network, live transport, or soak support MUST record unavailable, skipped, or denied evidence when required support is absent. Unsupported execution MUST NOT be treated as a passing profile.

#### Scenario: VM fault profile unavailable in CI
- GIVEN the CI host lacks support for executable VM fault injection
- WHEN the distributed CI matrix evaluates `vm-fault`
- THEN the matrix records unavailable evidence for that profile
- AND any broader gate either excludes it by explicit policy or denies claims requiring it.

### Requirement: Distributed CI gates have negative fixtures
r[molten.testing.distributed_ci.negative_fixtures] Molten SHOULD test distributed CI matrix validation with negative fixtures for missing shard artifacts, missing positive coverage, missing negative coverage, stale requirement refs, retry-only success, skipped VM support, and undeclared variance.

#### Scenario: Stale traceability ref fails
- GIVEN a traceability manifest entry points to a requirement id that no longer exists or a command that no longer produces the referenced artifact
- WHEN the distributed CI gate evaluates the manifest
- THEN validation fails closed with stale-reference diagnostics.

### Requirement: Distributed CI matrix is documented
r[molten.testing.distributed_ci.docs] User-facing documentation SHOULD describe distributed test profiles, commands, expected artifacts, reproducibility metadata, traceability gates, retry policy, unavailable handling, and evidence-only boundaries.

#### Scenario: Developer picks the right shard
- GIVEN a developer is changing distributed protocol logic
- WHEN they read the distributed testing docs
- THEN they can identify the smallest relevant profile to run before VM or soak checks and the evidence expected for release review.

### Requirement: VM fault descriptors are canonical
r[molten.testing.nixos_vm_fault_injection.fault_descriptors] Molten MUST define canonical VM fault descriptors for executable NixOS VM fault cases, including target node or link, fault kind, command profile, expected outcome, bounded duration or trigger condition, preflight refs, and evidence-only caveats.

#### Scenario: Fault descriptor binds target and expectation
- GIVEN a VM network partition fault targeting traffic from one node to another
- WHEN Molten canonicalizes the fault descriptor
- THEN the descriptor binds the source node, target node, fault kind, expected recovery or denial outcome, and preflight evidence refs.

### Requirement: Unsupported VM fault execution does not pass silently
r[molten.testing.nixos_vm_fault_injection.unavailable_boundary] Executable VM fault checks MUST fail closed or emit unavailable evidence when required KVM, QEMU, test-driver, network-control, filesystem, or privilege support is missing. Unsupported executable faults MUST NOT be converted into passing distributed-test evidence.

#### Scenario: Missing network-control support is unavailable
- GIVEN a CI host or VM image cannot execute the requested network fault injection
- WHEN the VM fault check requests that case
- THEN Molten records unavailable or deny evidence for that fault
- AND the final VM fault matrix does not count the case as pass evidence.

### Requirement: VM network faults are executable where supported
r[molten.testing.nixos_vm_fault_injection.network_faults] Molten SHOULD execute representative network delay, drop, one-way partition, rejoin, and asymmetric latency faults inside the NixOS VM topology when host and VM support are available, and bind resulting child workflow evidence into the VM fault receipt.

#### Scenario: Partition and rejoin preserve safety
- GIVEN two VM nodes with queued node-control or service workflow evidence
- WHEN an executable partition fault is injected and later removed
- THEN the resulting receipts show either idempotent recovery with matching operation refs or deny-before-side-effects diagnostics.

### Requirement: VM restart windows are exercised
r[molten.testing.nixos_vm_fault_injection.restart_windows] Molten MUST exercise crash, stop, or restart windows around queued control work, partial dispatch, duplicate send, service heartbeat, and receipt write/readback paths in at least one executable VM fault check.

#### Scenario: Duplicate send after restart is idempotent
- GIVEN a sender VM has emitted a send receipt for an operation
- WHEN the sender restarts and attempts the same send again
- THEN the receiver evidence shows duplicate suppression or idempotent replay
- AND no second semantic commit is accepted for the same operation id.

### Requirement: VM storage and state-root faults are bounded
r[molten.testing.nixos_vm_fault_injection.storage_state_faults] Molten SHOULD execute bounded storage and state-root fault cases such as missing artifacts, permission-denied state roots, corrupted diagnostic-only logs, or bounded disk pressure where deterministic VM support permits.

#### Scenario: Permission-denied state root fails before mutation
- GIVEN a VM node state root is made unwritable for a targeted operation
- WHEN the operation attempts to write control, ledger, or receipt state
- THEN Molten emits a denial or failure receipt before accepting side effects as pass evidence.

### Requirement: VM executable faults emit canonical receipts
r[molten.testing.nixos_vm_fault_injection.fault_receipts] Molten MUST emit canonical VM fault receipts that bind fault descriptor refs, host-support status, pre-fault refs, injection evidence refs, post-fault child refs, decisions, diagnostics, replay status, diagnostic log refs, and evidence-only caveats.

#### Scenario: Fault receipt binds pre and post evidence
- GIVEN an executable VM fault case completes
- WHEN the VM fault receipt is emitted
- THEN it identifies the fault descriptor, preflight evidence, injection evidence, observed child receipts, final decision, and any unavailable or degraded diagnostics.

### Requirement: VM executable fault validation has negative fixtures
r[molten.testing.nixos_vm_fault_injection.negative_fixtures] Molten SHOULD test VM executable fault validation with negative fixtures for unsupported host support, stale evidence refs, tampered fault descriptors, wrong topology membership, missing child refs, and log-only pass claims.

#### Scenario: Log-only pass is rejected
- GIVEN a VM fault run whose logs claim success but whose canonical fault receipt is missing or denied
- WHEN validation evaluates the run
- THEN validation rejects pass evidence and treats logs as diagnostic-only.

### Requirement: VM executable fault docs define boundaries
r[molten.testing.nixos_vm_fault_injection.docs] User-facing documentation SHOULD describe how to run executable VM fault checks, required host support, unavailable handling, authoritative receipt paths, diagnostic logs, and the limits of VM platform evidence.

#### Scenario: Operator inspects fault evidence
- GIVEN a realized VM fault check output
- WHEN an operator follows the documentation
- THEN they can identify the canonical fault receipts, unsupported-case diagnostics, child workflow refs, and evidence-only caveats without relying on raw build logs.

### Requirement: Distributed simulation direct fault fixtures are complete
r[molten.testing.distributed_simulation.direct_fault_fixtures] Molten SHOULD test every supported deterministic simulation fault class with a named fixture that asserts the expected decision, committed operation ids, denied operation ids, event kind, diagnostic, final-state ref, and run receipt ref stability.

#### Scenario: Benign faults preserve deterministic commits
- GIVEN admitted workflow commands under declared delay, drop, reorder, rejoin, crash, restart, or duplicate-delivery fault events
- WHEN the simulator runs twice with the same topology, scheduler profile, seed, commands, and fault plan
- THEN accepted operations emit stable event refs, final-state refs, diagnostics, and run receipt refs
- AND benign fault diagnostics name the active fault without granting authority, transport, policy, provenance, resource, source-gate, retention, deployment, or production-readiness trust.

#### Scenario: Denial faults stop before side effects
- GIVEN workflow commands exposed to stale-evidence, corrupted-receipt, resource-pressure, unauthorized-transport, undeclared-ambient-state, or partitioned-quorum fault events
- WHEN the simulator evaluates the commands
- THEN each affected command denies before side effects
- AND the run records denied operation ids, no semantic commit for denied commands, and fault-specific diagnostics in the canonical run receipt.

#### Scenario: Fixture drift changes canonical evidence
- GIVEN a passing direct fault fixture and a mutated peer id, operation id, fault kind, schedule field, payload ref, or required evidence ref
- WHEN the simulator canonicalizes both inputs
- THEN the mutated fixture changes the relevant topology, fault-plan, event, final-state, or run receipt ref
- AND any missing required evidence ref fails closed rather than reusing pass evidence.

### Requirement: Distributed simulation fixture traceability is explicit
r[molten.testing.distributed_simulation.fixture_traceability] Molten SHOULD bind the direct distributed simulation fixture set to traceability markers that identify positive and negative coverage commands, artifact refs or receipt refs, and the requirement ids covered by each fixture family.

#### Scenario: Fixture coverage names positive and negative evidence
- GIVEN distributed simulation requirements that claim direct fixture coverage
- WHEN traceability is scanned for release or review
- THEN positive fixtures and negative fixtures are both visible with command evidence
- AND missing, stale, unsupported, or diagnostic-only evidence cannot satisfy pass coverage.

### Requirement: Distributed CI profile wiring evidence follows configured profiles
r[molten.testing.distributed_ci.profile_wiring_evidence] Molten SHOULD test distributed CI metadata and gate fixtures against the configured distributed CI profile matrix. Profile ids, command surfaces, expected artifact kinds, cost classes, release-review statuses, retry policy, unavailable handling, and variance declarations MUST come from the configured matrix or an explicit reviewed fixture derived from it.

#### Scenario: Profile metadata follows the configured matrix
- GIVEN the configured distributed CI profile matrix
- WHEN metadata fixtures are built for fast, protocol, CLI, VM smoke, VM fault, and soak profiles
- THEN each metadata fixture binds the configured profile id, command surface, expected artifact kind, source or tree ref, test binary or package ref, topology ref, seed ref, fault-plan ref, receipt refs, variance refs, and diagnostic log refs
- AND profile metadata remains reproducible without reading ambient runtime state.

#### Scenario: Miswired profile evidence is denied
- GIVEN metadata for a missing profile id, mismatched command surface, missing receipt ref, missing variance declaration, unavailable required profile, or retry-only pass
- WHEN the distributed CI gate evaluates the run
- THEN the gate denies before accepting release pass evidence
- AND diagnostics identify the profile wiring error that must be fixed or explicitly exempted.

### Requirement: Distributed simulation uses generated fault interleaving properties
r[molten.testing.distributed_simulation.generated_fault_interleavings] Molten SHOULD use bounded generated properties to exercise combinations of topology, scheduler profile, command sequence, evidence refs, and fault-plan interleavings at the pure distributed simulation boundary.

#### Scenario: Generated benign interleavings remain deterministic
- GIVEN a generated topology, scheduler profile, deterministic seed, command sequence, and benign fault-plan interleaving within supported bounds
- WHEN the simulator runs the generated case more than once
- THEN run receipt refs, event refs, final-state refs, committed operation ids, and diagnostics remain stable
- AND duplicate, restart, crash, delay, drop, reorder, and rejoin behavior preserves the declared invariants.

#### Scenario: Generated denial interleavings fail before side effects
- GIVEN a generated command sequence with missing authority, unauthorized transport, stale evidence, corrupted receipt, resource pressure, ambient drift, or partitioned quorum inputs
- WHEN the simulator evaluates the generated case
- THEN affected commands deny before side effects
- AND denied operation ids and diagnostics are recorded in canonical evidence.

### Requirement: Generated failures preserve replayable seeds
r[molten.testing.distributed_simulation.generated_repro_seed] Molten MUST preserve enough generated-case data to replay or inspect a failing distributed simulation property without relying on ambient randomness, clocks, host paths, or process state.

#### Scenario: Failing generated case emits repro artifact
- GIVEN a generated distributed simulation property failure
- WHEN the test harness records the failure
- THEN the repro artifact binds seed, topology, scheduler profile, fault plan, commands, invariant name, diagnostics, and receipt refs
- AND the artifact is diagnostic-only unless a later gate validates it as pass or deny evidence.

#### Scenario: Replayed seed reproduces the same canonical refs
- GIVEN a generated-case repro artifact and the same simulator version
- WHEN the harness replays the stored seed and explicit inputs
- THEN the replay produces the same relevant topology, fault-plan, event, final-state, and run receipt refs or reports a schema/version mismatch.

### Requirement: NixOS VM evidence includes true cross-node live transport
r[molten.testing.nixos_vm.cross_node_live_transport] Molten SHOULD include a NixOS VM scenario where a sender node delivers a control request to a receiver node through the admitted live transport path before any test-driver artifact export is used for evidence collection.

#### Scenario: Sender reaches receiver through live transport
- GIVEN a VM topology with admitted sender and receiver nodes, a receiver live listener, a bound ticket, peer admission evidence, and authority evidence
- WHEN the sender submits a control request through the live transport path
- THEN the VM evidence binds send, receive, ingress, queue, dispatch, reconcile, ack, and protocol-gate receipts
- AND artifact copying is used only after the live exchange for evidence export and review.

#### Scenario: Live transport scope is explicit
- GIVEN a passing cross-node live transport VM run
- WHEN the run receipt is inspected
- THEN it states that the evidence is scoped to the NixOS VM topology
- AND it does not grant authority, policy, provenance, resource, source-gate, retention, deployment, or production-readiness trust by itself.

### Requirement: Live transport VM gate rejects stale or log-only evidence
r[molten.testing.nixos_vm.live_transport_negative_gate] Molten MUST reject cross-node live transport VM pass claims when the expected peer, expected node, ticket, receive receipt, protocol gate, or receipt chain is missing, stale, mismatched, or represented only by logs.

#### Scenario: Wrong peer or stale ticket denies
- GIVEN a cross-node live transport bundle with a wrong expected peer or stale ticket ref
- WHEN the VM transport gate evaluates the bundle
- THEN the gate emits deny evidence before accepting pass evidence
- AND diagnostics identify the stale or mismatched binding.

#### Scenario: Logs cannot replace receive receipt
- GIVEN diagnostic logs showing apparent live delivery but no canonical receive transport receipt
- WHEN the VM transport gate evaluates the run
- THEN the gate denies the pass claim because logs are diagnostic-only.

### Requirement: Multinode failure repro bundles are sealed evidence artifacts
r[molten.testing.multinode.failure_repro_bundle] Molten SHOULD export sealed multinode failure repro bundles that bind scenario fixture refs, topology refs, scheduler refs, seed refs, fault-plan refs, command refs, node evidence refs, receipt refs, diagnostics, log refs, redaction policy refs, replay status, and evidence-only caveats.

#### Scenario: Simulation failure bundle replays deterministically
- GIVEN a deterministic distributed simulation failure bundle with stored topology, scheduler, seed, fault plan, commands, and expected invariant
- WHEN the repro verifier replays the stored inputs
- THEN the replay produces the same relevant receipt refs or reports an explicit schema or version mismatch
- AND the bundle remains diagnostic evidence unless a separate gate validates a pass or deny claim.

#### Scenario: VM failure bundle verifies without pretending to replay
- GIVEN a VM failure bundle with platform observations and canonical receipts
- WHEN the repro verifier validates the seal and receipt bindings
- THEN the bundle can verify as non-replayable VM diagnostic evidence
- AND it must not claim deterministic replay if the inputs depend on live platform behavior.

### Requirement: Multinode repro bundles preserve privacy and fail closed
r[molten.testing.multinode.failure_repro_privacy_and_replay] Molten MUST reject tampered, unsealed, stale, private-without-reveal, missing-redaction, or diagnostic-only multinode repro bundles before materializing private content or accepting pass evidence.

#### Scenario: Tampered bundle fails verification
- GIVEN a sealed multinode repro bundle whose topology, fixture, receipt, or redaction manifest has been changed after sealing
- WHEN verify or unpack runs
- THEN verification fails closed before materializing bundle contents.

#### Scenario: Diagnostic bundle cannot satisfy pass gate
- GIVEN a verified failure repro bundle marked diagnostic-only
- WHEN a pass evidence gate evaluates the bundle
- THEN the gate rejects it as pass evidence even if logs appear successful.

### Requirement: Executable VM fault support matrix is explicit
r[molten.testing.nixos_vm.executable_fault_support_matrix] Molten SHOULD produce an executable VM fault support matrix that declares each fault kind, required capability, target node or link, command profile, expected outcome, host-support status, preflight refs, injection refs, child workflow refs, post-fault refs, diagnostics, diagnostic log refs, and caveats.

#### Scenario: Supported fault binds executable evidence
- GIVEN a VM fault descriptor whose required host or VM capability is available
- WHEN the VM injects the fault and validates the result
- THEN the receipt binds supported host status, pre-fault refs, injection refs, required child workflow refs, post-fault refs, diagnostics, and caveats
- AND the support matrix identifies the fault as executable evidence for the tested topology.

#### Scenario: Unsupported fault records unavailable evidence
- GIVEN a VM fault descriptor whose required host or VM capability is unavailable
- WHEN the VM fault check runs
- THEN the receipt records unavailable host support and diagnostic evidence
- AND unavailable execution does not satisfy pass evidence for that fault claim.

### Requirement: Executable VM fault validation rejects invalid claims
r[molten.testing.nixos_vm.executable_fault_validation_negatives] Molten MUST reject VM fault receipts that claim pass evidence without supported host status, required injection refs, required child workflow refs, matching topology, and canonical diagnostic evidence for denial or unavailable outcomes.

#### Scenario: Unsupported pass claim is rejected
- GIVEN a VM fault receipt that marks host support unavailable but claims a pass decision
- WHEN `fault-validate` evaluates the receipt
- THEN validation denies the receipt as an unsupported pass claim.

#### Scenario: Log-only pass claim is rejected
- GIVEN a VM fault descriptor and diagnostic log without the required canonical injection and child workflow refs
- WHEN `fault-validate` evaluates the evidence
- THEN validation denies before accepting the log as pass evidence.

### Requirement: Cross-node reconciliation gate binds distributed state refs
r[molten.testing.multinode.cross_node_reconciliation_gate] Molten SHOULD provide a cross-node reconciliation gate that compares explicit per-node evidence summaries against declared topology, scenario fixture, required receipt refs, expected equality classes, and allowed variance refs.

#### Scenario: Converged nodes pass reconciliation
- GIVEN a multinode run with per-node evidence summaries, matching topology refs, required workflow receipts, and declared equality classes
- WHEN the reconciliation gate evaluates the run
- THEN the gate emits a pass receipt binding node summaries, compared refs, allowed variance refs, diagnostics, and evidence-only caveats
- AND matching refs prove only the declared reconciliation scope.

#### Scenario: Declared variance is visible
- GIVEN a multinode run where selected per-node refs are allowed to differ
- WHEN the reconciliation gate evaluates those refs
- THEN the pass receipt binds the variance declaration that permits the difference
- AND undeclared differences remain denial conditions.

### Requirement: Reconciliation denies stale, missing, divergent, or log-only evidence
r[molten.testing.multinode.reconciliation_deny_drift] Molten MUST reject reconciliation pass claims when required node evidence is missing, stale, wrong-topology, duplicated, divergent without variance, or represented only by logs.

#### Scenario: Divergent queue ref denies
- GIVEN two node summaries that should share an expected queue or dispatch outcome but report different refs without declared variance
- WHEN the reconciliation gate evaluates the summaries
- THEN the gate denies before emitting pass evidence
- AND diagnostics identify the divergent ref class.

#### Scenario: Duplicate semantic commit denies
- GIVEN a multinode run where duplicate delivery produced more than one semantic commit for the same operation id
- WHEN reconciliation evaluates committed operation evidence
- THEN the gate denies the pass claim and identifies duplicate commit drift.

### Requirement: Local multiprocess multinode harness exercises real node processes
r[molten.testing.multinode.local_multiprocess_harness] Molten SHOULD provide a local multiprocess multinode harness that runs isolated `molten node` processes from an explicit scenario fixture and records canonical startup, workflow, shutdown, and run receipts.

#### Scenario: Cross-process control workflow records local integration evidence
- GIVEN a local multiprocess scenario fixture with separate node identities, isolated state roots, admitted local transport handles, and a valid control command
- WHEN the harness starts the node processes and runs the workflow
- THEN the run receipt binds the fixture ref, process-plan ref, startup refs, workflow receipt refs, shutdown refs, diagnostics, and evidence-only caveats
- AND the receipt states that local multiprocess evidence does not replace VM or production live evidence.

#### Scenario: Process planning stays deterministic
- GIVEN equivalent explicit process plans with the same node identities, state-root handles, command plan, and expected receipts
- WHEN the pure planner canonicalizes them
- THEN both plans produce the same plan ref without reading ports, process ids, clocks, or environment variables.

### Requirement: Local multiprocess harness isolates state and cleans up failures
r[molten.testing.multinode.process_isolation_cleanup] Molten MUST reject state-root collisions, transport-handle collisions, missing receipt bindings, stale tickets, and orphaned process or state evidence before accepting local multiprocess pass evidence.

#### Scenario: Collision fails before process start
- GIVEN a local multiprocess scenario where two nodes share a state-root handle or transport handle that must be isolated
- WHEN the harness validates the process plan
- THEN validation denies before starting the affected process
- AND diagnostics identify the colliding handle.

#### Scenario: Crash cleanup is recorded
- GIVEN a local multiprocess run where a child process crashes or is stopped during the workflow
- WHEN cleanup runs
- THEN the harness records cleanup or denial evidence
- AND no pass receipt is accepted unless required shutdown and cleanup receipts are present.

### Requirement: Multinode topology profile matrix is explicit
r[molten.testing.multinode.topology_profile_matrix] Molten SHOULD declare a multinode topology profile matrix that names the topology id, node roles, member set, allowed links, evidence scope, and required receipt classes for each distributed scenario family.

#### Scenario: Topology profile is bound into run evidence
- GIVEN a multinode scenario using a pairwise transport, control quorum, restart/rejoin, or subscriber topology profile
- WHEN metadata or run receipts are generated
- THEN the receipts bind the topology profile id and topology ref
- AND the evidence scope remains distinct from the execution cost profile.

#### Scenario: Review distinguishes topology claims
- GIVEN two runs with the same command surface but different topology profiles
- WHEN reviewers inspect the canonical metadata
- THEN each run identifies the role shape it covered
- AND neither run can satisfy claims outside its declared topology profile.

### Requirement: Role and membership negative fixtures deny confusion
r[molten.testing.multinode.role_membership_negatives] Molten MUST reject multinode evidence that treats undeclared nodes, undeclared links, subscriber peers, transport-only peers, or missing quorum evidence as admitted control-plane membership or authority evidence.

#### Scenario: Subscriber is not promoted to voter
- GIVEN a topology profile that declares a subscriber peer outside the Raft voting member set
- WHEN a command attempts to use subscriber evidence as voter membership evidence
- THEN the gate denies before accepting the command as control-plane pass evidence
- AND diagnostics name the role or membership mismatch.

#### Scenario: Wrong topology cannot satisfy pass evidence
- GIVEN a receipt from a topology profile whose nodes or links differ from the scenario fixture under review
- WHEN the multinode gate evaluates the receipt
- THEN the gate rejects the receipt as stale or wrong-topology evidence.

### Requirement: Multinode scenario fixtures are declarative and typed
r[molten.testing.multinode.declarative_scenario_fixtures] Molten SHOULD define typed, repository-owned multinode scenario fixtures that declare topology, profile, command surface, expected artifact kinds, deterministic seed, fault-plan refs, variance declarations, unavailable policy, and evidence-only caveats before execution.

#### Scenario: Valid fixture derives canonical metadata
- GIVEN a typed multinode scenario fixture with declared topology, profile, command surface, expected artifacts, receipt refs, variance refs, and caveats
- WHEN the testing harness validates the fixture and derives distributed CI metadata
- THEN the derived metadata binds the fixture values without reading ambient runtime state
- AND the fixture ref and metadata ref are stable for equivalent fixture content.

#### Scenario: Fixture authoring remains typed
- GIVEN a multinode scenario fixture authored in Nickel
- WHEN the fixture is exported for use by Rust validation or a NixOS VM check
- THEN the export must satisfy the repository-owned fixture contract before any pass evidence can be accepted.

### Requirement: Multinode scenario fixture validation fails closed
r[molten.testing.multinode.scenario_fixture_validation] Molten MUST reject multinode scenario fixtures that omit required topology, profile, receipt, variance, unavailable-policy, or artifact-kind bindings, or that claim unsupported execution as pass evidence.

#### Scenario: Missing or mismatched fixture fields deny
- GIVEN a fixture with a missing topology, missing command surface, stale receipt ref, undeclared variance, unsupported pass claim, or mismatched artifact kind
- WHEN the fixture validator evaluates it
- THEN validation denies before generating pass metadata
- AND diagnostics identify the invalid fixture binding.

#### Scenario: Diagnostic logs do not repair invalid fixtures
- GIVEN an invalid multinode scenario fixture and diagnostic logs that appear to show success
- WHEN the evidence gate evaluates the fixture
- THEN the gate rejects the fixture because canonical fixture and receipt bindings, not logs, determine pass evidence.

### Requirement: Composite distributed fault regressions are named and bounded
r[molten.testing.distributed_simulation.composite_fault_regression_suite] Molten SHOULD maintain a named deterministic composite fault regression suite for high-value distributed interleavings, including duplicate-after-restart, partition-with-stale-evidence, reorder-with-reconcile, crash-during-dispatch, and resource-pressure-during-quorum cases.

#### Scenario: Composite case binds deterministic inputs
- GIVEN a named composite fault case
- WHEN the simulation run receipt is emitted
- THEN it binds the case id, seed ref, topology ref, scheduler ref, fault-plan ref, command refs, invariant name, event refs, final-state ref, replay ref, diagnostics, and evidence-only caveats.

#### Scenario: Composite denial preserves no-side-effect evidence
- GIVEN a composite fault case expected to deny before side effects
- WHEN the simulation evaluates the case
- THEN denied operation ids, denial diagnostics, and final-state refs show that no semantic commit was accepted for the invalid operation.

### Requirement: Generated interleaving failures have promotion and budget evidence
r[molten.testing.distributed_simulation.generated_case_promotion_budget] Molten MUST require explicit promotion metadata, traceability coverage, profile eligibility, retry policy, variance declarations, and cost budget before a generated distributed case becomes a named regression fixture or release-review claim.

#### Scenario: Generated failure is promoted with stable refs
- GIVEN a generated distributed failure with stable seed, topology, scheduler, fault-plan, command, invariant, replay, and diagnostic refs
- WHEN it is promoted to a named regression fixture
- THEN the promotion evidence binds those refs and adds positive or negative traceability coverage for the new invariant.

#### Scenario: Retry-only success cannot satisfy composite pass evidence
- GIVEN a composite or generated case that only passes after a retry or undeclared variance
- WHEN the distributed evidence gate evaluates the case
- THEN the gate rejects it as deterministic pass evidence
- AND diagnostics identify retry-only or undeclared-variance status.

### Requirement: VM network-control capability is probed before network faults
r[molten.testing.nixos_vm_fault_injection.network_control_probe] Molten MUST record explicit VM network-control capability evidence before claiming executable network delay, drop, partition, rejoin, or asymmetric latency fault coverage.

#### Scenario: Supported network-control backend is recorded
- GIVEN a VM image and test-driver environment with a supported network-control backend
- WHEN the network fault preflight runs
- THEN the capability receipt binds the backend, target link, topology ref, cleanup strategy, and supported host status
- AND the executable fault may proceed only through that declared backend.

#### Scenario: Missing network-control backend remains unavailable
- GIVEN a VM image or host without a supported network-control backend
- WHEN a network fault is requested
- THEN the capability receipt records unavailable support
- AND the fault matrix does not count the case as pass evidence.

### Requirement: Executable VM network faults bind injection and cleanup evidence
r[molten.testing.nixos_vm_fault_injection.network_fault_executable_path] Molten SHOULD execute bounded VM network faults on capable hosts and MUST bind injection evidence, child workflow refs, cleanup evidence, topology refs, diagnostics, and caveats before accepting pass evidence.

#### Scenario: Partition and rejoin produce canonical evidence
- GIVEN a supported VM topology link and a declared cross-node workflow
- WHEN a partition fault is injected, observed, and removed
- THEN the fault receipt binds preflight, injection, child workflow, cleanup, and post-fault refs
- AND the resulting decision reflects idempotent recovery or deny-before-side-effects evidence.

#### Scenario: Missing cleanup denies pass
- GIVEN a network fault run with injection evidence but no cleanup evidence
- WHEN fault validation evaluates the receipt
- THEN validation denies before pass evidence is accepted
- AND diagnostics identify the missing cleanup boundary.

### Requirement: Local multiprocess runner starts real node processes
r[molten.testing.multinode.local_multiprocess_executable_runner] Molten SHOULD provide an executable local multiprocess runner that consumes a validated process plan, starts isolated `molten node` processes, runs a bounded cross-process workflow, and emits canonical startup, workflow, shutdown, cleanup, and run receipts.

#### Scenario: Local runner records cross-process evidence
- GIVEN a valid local multiprocess plan with isolated node ids, state-root handles, transport handles, expected receipt refs, and cleanup policy
- WHEN the runner starts the node processes and executes the workflow
- THEN the run receipt binds the plan ref, startup refs, workflow refs, shutdown refs, cleanup refs, diagnostics, and evidence-only caveats
- AND the receipt states that local multiprocess evidence is not VM evidence.

#### Scenario: Runner remains a thin shell over the pure plan
- GIVEN a local multiprocess execution request
- WHEN the runner prepares and executes the workflow
- THEN process spawning, filesystem writes, signal handling, and cleanup stay in the shell
- AND planning, receipt classification, and pass/deny decisions are testable as pure functions.

### Requirement: Local multiprocess runner fails closed on lifecycle and cleanup errors
r[molten.testing.multinode.local_multiprocess_runner_negatives] Molten MUST reject local multiprocess pass evidence when tickets are stale, state roots collide, transport handles collide, required receipts are missing, child processes orphan, timeouts occur, or cleanup evidence is absent.

#### Scenario: Missing workflow receipt denies pass evidence
- GIVEN a local multiprocess run where a node starts but the required workflow receipt is missing
- WHEN the run receipt is built
- THEN the decision denies before pass evidence is accepted
- AND diagnostics name the missing workflow receipt.

#### Scenario: Orphaned child process blocks pass
- GIVEN a local multiprocess run whose child process remains alive after cleanup
- WHEN cleanup validation runs
- THEN cleanup evidence records denial
- AND the final run receipt cannot pass.

### Requirement: NixOS multinode VM checks are sharded by scenario
r[molten.testing.nixos_vm_multinode.sharded_checks] Molten SHOULD expose named NixOS VM multinode shard checks whose scenario, command surface, required receipts, expected artifact kinds, unavailable policy, diagnostic logs, and caveats are declared before execution.

#### Scenario: Shard receipt binds the scenario boundary
- GIVEN a VM shard plan for smoke, live control, service/job coordination, restart recovery, or VM fault evidence
- WHEN the shard check completes
- THEN the shard receipt binds the scenario fixture ref, topology ref, node evidence refs, required child refs, diagnostic log refs, unavailable status, and evidence-only caveats
- AND the receipt states the shard evidence scope.

#### Scenario: Shard failure localizes the broken layer
- GIVEN a VM shard whose required canonical receipt is missing, denied, stale, or represented only by logs
- WHEN the shard receipt is generated
- THEN the shard decision denies or records unavailable according to the declared policy
- AND diagnostics name the missing or invalid receipt class.

### Requirement: NixOS multinode aggregate preserves child shard evidence
r[molten.testing.nixos_vm_multinode.shard_aggregate] Molten MUST treat the full multinode VM check as an aggregate over passing shard receipts and MUST NOT convert unavailable, skipped, denied, stale, or log-only child evidence into pass evidence.

#### Scenario: Aggregate binds every required shard
- GIVEN passing shard receipts for the declared VM shard matrix
- WHEN the aggregate VM evidence is emitted
- THEN the aggregate receipt binds each child shard ref, the topology ref, the package ref, and the manifest ref
- AND the aggregate remains review evidence over child receipts.

#### Scenario: Missing shard prevents aggregate pass
- GIVEN a full VM aggregate where a required shard receipt is absent or denied
- WHEN aggregate validation runs
- THEN the aggregate denies before pass evidence is accepted
- AND diagnostic logs cannot repair the missing child receipt.

### Requirement: Three-node VM topology covers quorum and restart/rejoin
r[molten.testing.multinode.three_node_quorum_topology] Molten SHOULD include a bounded three-node VM topology profile that exercises voter membership, majority/minority quorum behavior, restart/rejoin, and duplicate semantic commit suppression with canonical VM evidence.

#### Scenario: Majority evidence passes with matching node summaries
- GIVEN a three-node VM topology with declared voter roles, membership refs, quorum refs, and required workflow receipts
- WHEN a majority workflow completes
- THEN the VM evidence binds topology membership, quorum, per-node summaries, reconciliation refs, and child workflow refs
- AND the reconciliation gate passes only for matching semantic commit evidence.

#### Scenario: Restarting member rejoins without duplicate commit
- GIVEN a three-node VM topology where one member restarts after a queued operation
- WHEN the member rejoins and the workflow is reconciled
- THEN the evidence shows idempotent recovery or duplicate suppression
- AND no second semantic commit is accepted for the same operation id.

### Requirement: Three-node VM negatives reject role and quorum confusion
r[molten.testing.multinode.three_node_membership_negatives] Molten MUST reject three-node VM pass claims that treat subscriber, observer, transport-only, partitioned-minority, or missing-quorum evidence as admitted voter membership or authority evidence.

#### Scenario: Subscriber cannot satisfy voter membership
- GIVEN a three-node VM scenario where a subscriber or observer receipt is supplied as voter membership evidence
- WHEN membership or reconciliation validation runs
- THEN the gate denies before pass evidence is accepted
- AND diagnostics name the role mismatch.

#### Scenario: Partitioned minority cannot satisfy quorum
- GIVEN a three-node VM topology with only minority-side receipts after a partition
- WHEN quorum validation evaluates the evidence
- THEN validation denies the quorum claim
- AND diagnostic logs cannot substitute for missing majority receipts.

### Requirement: VM failures export sealed diagnostic repro bundles
r[molten.testing.multinode.vm_failure_repro_export] Molten SHOULD export sealed diagnostic failure repro bundles for VM multinode shard or aggregate failures, binding scenario, topology, node evidence, child receipts, validation receipts, diagnostic logs, replay status, redaction policy, and evidence-only caveats.

#### Scenario: Denied VM shard produces diagnostic bundle
- GIVEN a VM shard with denied, unavailable, or failed validation evidence
- WHEN failure repro export runs
- THEN the bundle binds the scenario fixture ref, topology ref, node summary refs, child receipt refs, diagnostic log refs, validation refs, replay status, and caveats
- AND the bundle is marked diagnostic-only.

#### Scenario: VM live observation is not replayable by default
- GIVEN a VM failure bundle containing unrecorded live transport observations
- WHEN the bundle is verified
- THEN verification records non-replayable diagnostic status
- AND the bundle cannot satisfy deterministic pass evidence.

### Requirement: VM failure repro bundles fail closed on privacy, tamper, and pass-gate use
r[molten.testing.multinode.vm_failure_repro_privacy_gate] Molten MUST reject VM failure repro bundles that are tampered, unsealed, stale, private without matching reveal receipts, missing redaction policy, or presented as pass evidence.

#### Scenario: Tampered VM failure bundle is rejected
- GIVEN a sealed VM failure repro bundle whose topology, node summary, child receipt, diagnostic ref, or seal metadata has been modified
- WHEN verification runs
- THEN verification denies before materializing bundle contents
- AND diagnostics identify the stale or tampered binding.

#### Scenario: Diagnostic bundle cannot pass gate
- GIVEN a verified VM failure repro bundle
- WHEN a pass evidence gate evaluates it
- THEN the gate rejects it as diagnostic-only evidence
- AND no diagnostic log can override the canonical failure decision.

### Requirement: VM scenarios bind declarative fixture metadata
r[molten.testing.multinode.vm_scenario_metadata_gate] Molten SHOULD bind each VM multinode shard or aggregate run to validated multinode scenario fixture metadata before accepting the run as VM pass evidence.

#### Scenario: VM run consumes checked scenario metadata
- GIVEN a VM run and a checked multinode scenario fixture export with topology, profile, command surface, expected artifact kinds, variance refs, unavailable policy, and caveats
- WHEN the VM evidence is validated
- THEN the VM run binds the scenario metadata ref and fixture ref
- AND diagnostics identify any mismatch between the scenario declaration and the observed VM evidence.

#### Scenario: Wrong fixture cannot satisfy VM pass evidence
- GIVEN VM receipts from one scenario and metadata from a different scenario fixture
- WHEN the VM scenario gate evaluates them
- THEN the gate denies before pass evidence is accepted
- AND diagnostic logs cannot repair the mismatch.

### Requirement: VM evidence passes multinode reconciliation gates
r[molten.testing.multinode.vm_reconciliation_gate] Molten MUST run multinode topology membership, reconciliation, and live transport gates over VM evidence before a VM run claims cross-node topology, reconciliation, or live transport success.

#### Scenario: Reconciled VM nodes produce gate evidence
- GIVEN VM node summaries with matching topology refs, scenario fixture refs, required receipt refs, queue refs, ledger refs, dispatch refs, ack refs, protocol refs, and declared variance refs
- WHEN the VM reconciliation gate evaluates the summaries
- THEN it emits a passing reconciliation receipt bound into the VM shard or aggregate manifest.

#### Scenario: Divergent VM evidence denies without declared variance
- GIVEN VM node summaries with divergent queue, ledger, dispatch, ack, protocol, or semantic commit refs and no matching variance declaration
- WHEN the VM reconciliation gate evaluates the summaries
- THEN it denies before pass evidence is accepted
- AND the diagnostic names the divergent equality class.

### Requirement: Contract invariants have positive and negative fixtures
r[molten.testing.contract_negative_fixtures.invariant_matrix] Each repository-owned Nickel contract module SHOULD provide positive fixtures for reviewed valid exports and focused negative fixtures for every exported field-domain or cross-field invariant, or record an explicit fixture exemption.

#### Scenario: Valid contract fixture exports
- GIVEN a contract module with reviewed valid source fixtures
- WHEN fixture validation runs
- THEN every positive fixture exports successfully and matches the reviewed generated artifact when one is checked in

#### Scenario: Invalid contract fixture fails
- GIVEN a negative fixture that violates exactly one documented field-domain or cross-field invariant
- WHEN fixture validation runs
- THEN the fixture fails before generated JSON or Preserves evidence is refreshed

### Requirement: Negative fixture failure classes are reviewable
r[molten.testing.contract_negative_fixtures.failure_classes] Negative contract fixtures SHOULD name the expected failure class or invariant so a fixture that fails for the wrong reason remains visible during review.

#### Scenario: Fixture fails for expected invariant
- GIVEN a negative fixture named for a malformed ref, duplicate id, missing evidence, invalid enum, stale metadata, or cross-field contradiction
- WHEN validation reports the failure
- THEN reviewers can identify the intended invariant rather than treating any failure as sufficient

### Requirement: Consensus fault matrix is deterministic
r[molten.testing.consensus_fault_matrix] Molten SHOULD include deterministic consensus simulation fixtures for failed leader, slow leader, concurrent proposals, majority partition progress, minority partition denial, stale linearizable read denial, and local-stale read classification. Fixtures MUST bind topology refs, algorithm profile refs, membership refs, fault-plan refs, operation ids, expected decisions, final-state refs, and receipt refs.

#### Scenario: Majority-connected control plane makes progress
- GIVEN a deterministic simulation with an admitted consensus profile, a declared fault plan, and a majority-connected set of replicas
- WHEN a valid control-plane command is proposed through an admitted path
- THEN the simulation emits a pass receipt with committed operation ids, final-state ref, quorum evidence, and fault diagnostics.

#### Scenario: Minority partition denies progress
- GIVEN a deterministic simulation where a replica or partition cannot reach an admitted majority
- WHEN a linearizable read or mutating control-plane command is attempted
- THEN the simulation emits denial evidence before semantic commit
- AND diagnostics name the missing majority or freshness evidence.

#### Scenario: Stale read classification is stable
- GIVEN a replica has lagging local state and a client requests either linearizable or local-stale read behavior
- WHEN the simulation evaluates the read
- THEN the linearizable read denies unless freshness evidence is present
- AND the local-stale read emits a stable non-authoritative receipt classification.

### Requirement: Leaderless experimental fixtures cover positive and negative paths
r[molten.testing.leaderless_experimental_fixtures] If Molten implements an experimental leaderless quorum profile, Molten MUST include deterministic fixtures showing majority-connected non-leader proposal progress, concurrent proposal convergence, denied minority proposals, denied missing experimental evidence, and denied production admission without accepted policy/proof/simulation evidence.

#### Scenario: Non-leader proposal can commit experimentally
- GIVEN an admitted experimental leaderless simulation profile and a majority-connected non-leader replica
- WHEN the replica proposes a valid control-plane command
- THEN the command commits only through the profile's quorum rule
- AND the receipt records the proposer, quorum evidence, final-state ref, and experimental caveat.

#### Scenario: Concurrent proposals converge or deny deterministically
- GIVEN multiple replicas propose concurrent commands for the same decision point under the experimental profile
- WHEN the deterministic scheduler explores the declared ordering
- THEN the simulation either decides one canonical outcome with matching replica state refs or denies unsafe progress
- AND no fixture accepts divergent decided values for the same slot or log position.

#### Scenario: Experimental evidence missing denies production admission
- GIVEN a manifest requests production use of the experimental leaderless profile without accepted proof/model, policy, simulation, placement, or membership evidence
- WHEN gate validation evaluates the manifest
- THEN admission denies
- AND diagnostics state which evidence class is missing.

### Requirement: Consensus placement fixtures cover safe and unsafe plans
r[molten.testing.consensus_placement_fixtures] Molten SHOULD include placement fixtures for admitted fault-domain placement, missing placement evidence, unsafe member concentration, membership-policy drift, stale placement refs, and latency-diagnostic readback.

#### Scenario: Admitted placement binds group evidence
- GIVEN a consensus group placement plan satisfies configured fault-domain and membership policy
- WHEN the placement fixture renders evidence
- THEN the placement report binds selected members, policy refs, membership refs, majority-reachability assumptions, and diagnostics
- AND the group manifest can reference that placement report.

#### Scenario: Unsafe placement is rejected
- GIVEN a placement plan has missing evidence, stale membership refs, disallowed concentration, or policy drift
- WHEN group installation or placement validation runs
- THEN Molten denies the placement before group installation
- AND diagnostics identify the unsafe placement reason.

### Requirement: Consensus engine conformance fixtures are deterministic
r[molten.testing.consensus_engine_conformance] Molten MUST include deterministic conformance fixtures for each registered consensus engine profile. Fixtures MUST cover admitted proposal, duplicate operation denial, linearizable read freshness, local-stale read classification, snapshot and recovery, membership/config transition denial, canonical application-state replay, and normalized receipt shape.

#### Scenario: Registered engine passes conformance suite
- GIVEN a registered consensus engine profile with complete test fixture inputs
- WHEN the deterministic conformance suite runs for that profile
- THEN the suite emits pass receipts for proposal, read, snapshot, recovery, membership/config, and canonical replay cases
- AND each receipt binds engine profile, profile version, fixture id, input refs, final-state ref, and normalized evidence fields.

#### Scenario: Divergent application replay fails conformance
- GIVEN an engine produces a final application state ref that differs from canonical replay for the same command sequence
- WHEN the deterministic conformance suite compares expected and actual state refs
- THEN the suite fails the engine profile conformance receipt
- AND diagnostics identify the fixture id, command sequence ref, expected state ref, and actual state ref.

### Requirement: Consensus registry negative fixtures fail closed
r[molten.testing.consensus_registry_negative_fixtures] Molten MUST include negative fixtures for unknown engine profile, disabled engine profile, experimental profile requested for production, missing conformance refs, missing proof/model evidence, unsupported read consistency mode, mismatched profile version, missing placement requirements, and unsupported membership/config capability.

#### Scenario: Unknown profile fixture denies runtime construction
- GIVEN a fixture manifest names an unknown consensus engine profile
- WHEN runtime construction resolves the profile through the engine registry
- THEN the fixture emits denial evidence before opening control-plane state
- AND diagnostics identify the unsupported profile without fallback.

#### Scenario: Missing evidence fixture denies production admission
- GIVEN a registry entry lacks required conformance, proof/model, placement, membership, or policy evidence
- WHEN engine admission policy evaluates production status
- THEN the fixture emits denial evidence
- AND diagnostics name each missing evidence class.

### Requirement: Consensus switchover fixtures cover safe and unsafe transitions
r[molten.testing.consensus_switchover_fixtures] Molten SHOULD include deterministic switchover fixtures for safe source-to-target bootstrap, stale source-state denial, target admission denial, incompatible membership denial, placement drift denial, failed replay/conformance denial, stale writer fencing, and target read denial before activation.

#### Scenario: Safe switchover fixture activates target epoch
- GIVEN a source engine state ref, target engine profile, compatible membership and placement evidence, replay/conformance evidence, and operator approval refs
- WHEN the switchover fixture evaluates the plan
- THEN it emits a committed switchover receipt with target engine epoch, target bootstrap state ref, currentness evidence, and rollback posture
- AND subsequent normalized reads use the activated target epoch.

#### Scenario: Stale writer fixture denies after activation
- GIVEN a switchover fixture has activated a target engine epoch
- WHEN a delayed source-engine write receipt from the prior epoch is replayed
- THEN the fixture denies mutation authority for that receipt
- AND diagnostics identify the superseded engine epoch.

### Requirement: Modularity rules are reviewed policy
r[molten.modularity.boundary_gates.policy] Repository dependency-boundary rules SHOULD be declared in reviewed source-controlled policy that names each rule, owning layer, allowed or denied dependency patterns, diagnostic guidance, and exemption class.

#### Scenario: Valid boundary policy loads
- GIVEN a reviewed dependency-boundary policy with unique rule ids and valid path patterns
- WHEN the boundary validator loads the policy
- THEN validation succeeds and preserves the reviewed rules deterministically

#### Scenario: Malformed boundary policy fails
- GIVEN duplicate rule ids, unknown layers, invalid path patterns, or contradictory allow/deny entries
- WHEN the boundary validator loads the policy
- THEN validation fails before generated policy or release evidence is refreshed

### Requirement: Boundary validator reports actionable diagnostics
r[molten.modularity.boundary_gates.validator] The dependency-boundary validator MUST report deterministic diagnostics that identify the rule id, source file, forbidden target or pattern, and remediation or exemption guidance for each violation.

#### Scenario: Forbidden dependency is reported
- GIVEN a source file imports a dependency forbidden by its layer rule
- WHEN the boundary validator scans the repository
- THEN it reports the violating file, the forbidden target, the rule id, and the expected remediation or exemption class

#### Scenario: Clean source passes
- GIVEN source files whose imports satisfy the reviewed boundary policy
- WHEN the boundary validator scans the repository
- THEN it reports a pass decision with no violation diagnostics

### Requirement: Boundary gate has positive and negative fixtures
r[molten.modularity.boundary_gates.fixtures] Boundary-gate validation SHOULD include positive fixtures for allowed imports and negative fixtures for representative forbidden dependencies and malformed policy inputs.

#### Scenario: Positive fixture passes
- GIVEN a fixture representing allowed core, codec, runtime, adapter, and CLI import relationships
- WHEN boundary validation runs on the fixture
- THEN the fixture passes without diagnostics

#### Scenario: Negative fixture fails for expected rule
- GIVEN a fixture representing core-to-adapter, runtime-to-CLI, codec-to-domain, or unclassified-public-export violation
- WHEN boundary validation runs on the fixture
- THEN it fails with the expected rule id and does not pass because of unrelated parser or policy errors

### Requirement: Boundary gate is runnable as focused validation
r[molten.modularity.boundary_gates.integration] The dependency-boundary gate SHOULD be runnable as a focused validation command or documented check and MAY later be wired into Nix, Octet, Cairn release-readiness, or CI evidence rails.

#### Scenario: Developer runs focused boundary check
- GIVEN a checkout with boundary policy and validator fixtures
- WHEN a developer runs the documented focused boundary command
- THEN the command checks the configured source scope and emits pass or violation diagnostics suitable for release review

### Requirement: Harness responsibilities are layered
r[molten.testing.modularity.harness_layers] Harness implementation SHOULD separate schema models, pure gate decisions, fixture builders, canonical receipt construction, and IO or CLI shells.

#### Scenario: Harness module ownership is clear
- GIVEN harness schema or gate code is reorganized
- WHEN reviewers inspect the module layout
- THEN each module has an identifiable responsibility such as schema, decision, fixtures, receipts, or shell

### Requirement: Gate decisions are pure
r[molten.testing.modularity.pure_gate_decisions] Harness gate decisions SHOULD be deterministic functions over typed suite, report, policy, and evidence inputs, without filesystem reads, CLI rendering, process execution, or adapter IO.

#### Scenario: Valid report passes in memory
- GIVEN a valid suite/report input represented in memory
- WHEN the gate decision core evaluates it
- THEN it returns a pass decision and structured receipt input without reading files or running commands

#### Scenario: Malformed report denies in memory
- GIVEN a malformed, stale, unsupported, or contradictory suite/report input represented in memory
- WHEN the gate decision core evaluates it
- THEN it returns a deny or diagnostic result without writing evidence or invoking the CLI shell

### Requirement: Runtime code consumes harness evidence, not harness orchestration
r[molten.testing.modularity.runtime_boundary] Runtime modules MUST NOT depend on harness runners or release-test orchestration to make normal runtime decisions; they MAY consume canonical gate receipts or evidence summaries as explicit inputs.

#### Scenario: Runtime consumes receipt summary
- GIVEN runtime admission depends on prior harness evidence
- WHEN the runtime core evaluates admission
- THEN it consumes a canonical receipt or typed evidence summary rather than invoking harness suite execution

### Requirement: Harness modularity has positive and negative fixtures
r[molten.testing.modularity.fixtures] Harness schema or gate refactors SHOULD include positive fixtures for valid inputs and negative fixtures for malformed, stale, unsupported, or contradictory inputs.

#### Scenario: Fixture matrix covers gate behavior
- GIVEN a harness gate boundary is extracted
- WHEN focused validation runs
- THEN valid fixtures pass and negative fixtures fail for the expected invariant class

### Requirement: Boundary coverage is gateable
r[molten.testing.boundary_coverage.gate] Molten SHOULD provide a boundary coverage gate that evaluates harness reports or traceability receipts for exercised runtime boundary classes and emits canonical pass, deny, or exempt diagnostics.

#### Scenario: Unexercised policy denial is reported
- GIVEN a suite that exercises policy pass paths but no policy denial path
- WHEN the boundary coverage gate evaluates the report for a requirement that needs denial coverage
- THEN the gate denies or reports a missing policy-denial boundary diagnostic

### Requirement: Positive and negative boundary classes are tracked
r[molten.testing.boundary_coverage.positive_negative] Evidence-bearing requirements SHOULD declare or derive both positive and negative boundary coverage expectations unless an explicit exemption applies.

#### Scenario: Capability coverage includes grant and deny
- GIVEN a requirement covering capability admission
- WHEN boundary coverage is summarized
- THEN the summary identifies both admitted capability behavior and denied capability behavior or an explicit exemption

### Requirement: Boundary coverage exemptions are explicit
r[molten.testing.boundary_coverage.exemptions] Boundary coverage exemptions MUST carry reason class, evidence path or receipt ref, scope, and diagnostic-only caveats, and MUST NOT silently satisfy behavioral pass evidence.

#### Scenario: VM-unavailable exemption remains visible
- GIVEN a boundary class that requires VM support unavailable on the current host
- WHEN the boundary coverage gate evaluates the profile
- THEN it records an unavailable or exempt diagnostic without converting the missing VM boundary into pass evidence

### Requirement: CI test runs emit canonical receipts
r[molten.testing.ci_run_receipt.canonical_receipt] Molten SHOULD emit a canonical CI test-run receipt for nextest-backed CI checks that binds source ref, profile id, command surface, nextest config ref, Cargo metadata ref, binaries metadata ref, rendered JUnit ref, counts, decision, diagnostics, and caveats.

#### Scenario: CI receipt binds nextest artifacts
- GIVEN a successful nextest-backed CI check
- WHEN the CI test-run receipt is emitted
- THEN the receipt binds the source ref, profile id, Cargo metadata ref, binaries metadata ref, nextest config ref, JUnit ref, counts, and pass decision

### Requirement: Nix nextest output includes receipt binding
r[molten.testing.ci_run_receipt.nix_nextest_binding] Nix nextest checks SHOULD preserve the canonical CI test-run receipt beside existing metadata and rendered JUnit outputs.

#### Scenario: Nix output has canonical readback
- GIVEN the Nix nextest check succeeds
- WHEN a reviewer inspects the output path
- THEN the output contains the canonical CI receipt and the metadata or JUnit refs named by that receipt

### Requirement: JUnit remains a rendered view
r[molten.testing.ci_run_receipt.junit_view_only] JUnit output MUST be treated as a rendered view over test execution evidence and MUST NOT satisfy CI pass evidence without the required canonical metadata or receipt binding.

#### Scenario: JUnit-only output is insufficient
- GIVEN a JUnit file with passing test cases but missing Cargo metadata or CI receipt binding
- WHEN CI evidence is evaluated for release readback
- THEN the evidence is denied or marked incomplete rather than accepted as pass evidence

### Requirement: Missing CI metadata denies
r[molten.testing.ci_run_receipt.deny_on_missing_metadata] CI receipt validation MUST fail closed when required metadata, profile identity, rendered output refs, counts, or decision fields are missing, stale, or mismatched.

#### Scenario: Stale binaries metadata is rejected
- GIVEN a CI receipt whose binaries metadata ref no longer matches the preserved binaries metadata file
- WHEN the receipt is validated
- THEN validation denies with a binaries-metadata binding diagnostic

### Requirement: Generated tamper cases
r[molten.testing.tamper_matrix.generated_cases] Molten SHOULD provide a reusable generated or table-driven tamper matrix for evidence artifacts whose parsers or gates accept pass evidence.

#### Scenario: Matrix generates stale-ref case
- GIVEN a valid harness gate receipt fixture
- WHEN the tamper matrix generates a stale subject-ref case
- THEN the resulting fixture preserves the original control metadata and identifies the expected stale-ref denial class

### Requirement: Tampered evidence fails closed
r[molten.testing.tamper_matrix.fail_closed] Parsers and gates exercised by the tamper matrix MUST reject mutated evidence before emitting pass evidence and MUST preserve canonical diagnostics for the denial class.

#### Scenario: Tampered embedded receipt is denied
- GIVEN a valid sealed repro bundle whose embedded gate receipt is changed by the tamper matrix
- WHEN the bundle gate evaluates the mutated bundle
- THEN the gate denies before accepting pass evidence and records a receipt or seal diagnostic

### Requirement: Tamper matrix coverage is traceable
r[molten.testing.tamper_matrix.coverage] Tamper matrix coverage SHOULD be recorded in the checked-in evidence matrix or traceability receipts for the requirements it protects.

#### Scenario: Requirement lists tamper coverage
- GIVEN a requirement that depends on fail-closed bundle validation
- WHEN the evidence matrix is rendered
- THEN it lists the positive control fixture and the generated negative tamper cases that cover the requirement

### Requirement: Hegel counterexamples become replay fixtures
r[molten.testing.hegel_counterexample.replay_fixture] Hegel property failures SHOULD emit canonical counterexample fixtures that bind the property id, generator profile ref, generation seed, shrink path, final shrunk Preserves input, replay identity, trace refs, receipt refs, and diagnostics.

#### Scenario: Shrunk counterexample replays without generator
- GIVEN a Hegel property failure with a final shrunk input
- WHEN the harness writes a counterexample fixture
- THEN another run can replay the fixture from canonical data without invoking the property generator

### Requirement: Counterexample promotion is reviewed
r[molten.testing.hegel_counterexample.promotion] Promotion from a counterexample fixture to a deterministic regression case or known-deny fixture MUST record review metadata, source property refs, old and new fixture refs, reason class, and resulting status.

#### Scenario: Counterexample becomes regression
- GIVEN a reviewed Hegel counterexample fixture for a fixed bug
- WHEN it is promoted to a deterministic regression suite
- THEN the promotion record binds the source fixture ref, new suite entry ref, review reason, and post-fix pass evidence

### Requirement: Counterexample exports preserve confidentiality
r[molten.testing.hegel_counterexample.redaction] Counterexample fixture export MUST redact or encrypt sensitive generated inputs and capability-bearing traces before materializing shareable repro evidence.

#### Scenario: Sensitive generated input is redacted
- GIVEN a property failure whose shrunk Preserves input contains a secret marker
- WHEN the counterexample fixture is exported without reveal authority
- THEN the exported fixture uses redaction markers or encrypted refs and records transform evidence

### Requirement: CLI harness assertions are receipt-first
r[molten.testing.cli_receipt_first.normative_artifacts] Evidence-bearing CLI harness tests MUST assert canonical Preserves artifacts or receipts as the normative result of a command before relying on rendered stdout or stderr.

#### Scenario: Gate check assertion parses receipt
- GIVEN a CLI test runs `molten test gate check` on a valid report
- WHEN the command emits a gate receipt
- THEN the test parses the canonical receipt and asserts decision, artifact kind, report ref, suite ref, and gate checks

### Requirement: Rendered CLI output is diagnostic-only
r[molten.testing.cli_receipt_first.stdout_diagnostic_only] CLI stdout, stderr, markdown, JSON, JUnit, or terminal summaries SHOULD be tested only as rendered views over canonical artifacts, not as the sole evidence-bearing oracle.

#### Scenario: Summary string does not replace receipt
- GIVEN a command prints a human summary containing a pass decision
- WHEN no parseable canonical receipt or report is produced for an evidence-bearing path
- THEN the CLI harness test does not accept the summary as normative pass evidence

### Requirement: CLI negative cases fail closed with canonical artifacts
r[molten.testing.cli_receipt_first.negative_fail_closed] CLI harness negative tests MUST verify malformed, stale, missing, diagnostic-only, or unauthorized evidence fails closed and emits a canonical failure or deny artifact when the command supports one.

#### Scenario: Diagnostic-only bundle is rejected by pass gate
- GIVEN a diagnostic-only repro bundle
- WHEN a CLI test runs a pass-evidence gate command against it
- THEN the command denies before emitting pass evidence and the test asserts the canonical failure or deny artifact

### Requirement: Evidence suites have replay smoke coverage
r[molten.testing.replay_smoke.all_evidence_suites] Evidence-bearing deterministic harness suites SHOULD have replay smoke coverage that runs the suite, replays it from recorded effects when applicable, and reruns it from fresh declared fixtures.

#### Scenario: Deterministic suite smoke passes
- GIVEN an evidence-bearing deterministic suite with declared fixtures and effect records
- WHEN replay smoke executes run, replay, and fresh rerun
- THEN the canonical report refs, final-state refs, effect-log refs, and required trace or receipt refs match the declared replay identity

### Requirement: Fresh reruns compare canonical refs
r[molten.testing.replay_smoke.fresh_rerun] Replay smoke comparisons MUST use canonical refs and receipts rather than rendered logs, wall-clock timing, temporary paths, or process ids.

#### Scenario: Temporary path variance is ignored only when declared
- GIVEN a fresh rerun produces a different temporary diagnostic path but the same semantic report and final-state refs
- WHEN replay smoke compares the runs
- THEN the semantic refs match and the path variance is ignored only if an explicit variance declaration exists

### Requirement: Non-replayable suites are excluded visibly
r[molten.testing.replay_smoke.non_replayable_excluded] Suites marked exploratory, live-only, unavailable, or non-replayable MUST be excluded from deterministic pass evidence and SHOULD emit a visible replay-smoke diagnostic.

#### Scenario: Live-only run cannot satisfy deterministic gate
- GIVEN a live-only diagnostic run without a recorded effect log
- WHEN replay smoke evaluates it for deterministic evidence
- THEN the run is excluded with a non-replayable diagnostic even if its rendered status is pass

### Requirement: Semantic test profiles
r[molten.testing.nextest.semantic_profiles] Molten SHOULD expose semantic test profiles for fast core, harness, CLI, distributed simulation, VM/platform, and dogfood or soak evidence scopes.

#### Scenario: Developer selects smallest useful profile
- GIVEN a change that affects deterministic harness replay behavior
- WHEN a developer inspects the semantic profile matrix
- THEN the matrix identifies the harness-focused command and its expected evidence artifacts before VM or dogfood checks are required

### Requirement: Profile evidence scope is explicit
r[molten.testing.nextest.risk_scope] Each semantic profile MUST declare its evidence scope, command surface, retry policy, expected artifact kinds, cost class, and release-review caveats.

#### Scenario: Distributed simulation does not claim VM evidence
- GIVEN a passing distributed simulation profile run
- WHEN release evidence is summarized
- THEN the profile scope identifies it as deterministic simulation evidence and does not claim VM, live transport, or dogfood readiness evidence

### Requirement: Profile outputs are preserved by Nix checks
r[molten.testing.nextest.nix_outputs] Nix-backed profile checks SHOULD preserve deterministic metadata and rendered JUnit outputs for the selected semantic profile.

#### Scenario: Harness profile emits readback artifacts
- GIVEN the harness semantic profile runs through a Nix check
- WHEN the check succeeds
- THEN the output contains profile metadata, rendered JUnit when configured, and canonical refs or receipts needed for readback

### Requirement: Exploratory retries are excluded from deterministic evidence
r[molten.testing.nextest.exploratory_exclusion] Exploratory profiles MAY allow retries for diagnostics, but retry success MUST NOT satisfy deterministic CI, release, admission, or upgrade evidence gates.

#### Scenario: Retry-only pass remains diagnostic
- GIVEN an exploratory profile passes only after a retry
- WHEN deterministic evidence gates evaluate the run
- THEN the run is excluded as deterministic pass evidence and preserved only as diagnostic evidence

### Requirement: Checked-in test evidence matrix
r[molten.testing.evidence_matrix.checked_in_manifest] Molten SHOULD maintain a checked-in requirement-to-test evidence matrix for testing-harness requirements, with typed entries for requirement ids, coverage kinds, targets, commands, artifact refs, evidence scope, and caveats.

#### Scenario: Reviewer inspects matrix coverage
- GIVEN a testing-harness requirement that is implemented or changed
- WHEN a reviewer inspects the checked-in matrix
- THEN the matrix identifies the requirement's positive evidence, negative evidence, and any property, CLI, integration, or exemption evidence entries

### Requirement: Changed requirements require positive and negative evidence
r[molten.testing.evidence_matrix.changed_requirement_gate] The matrix gate MUST fail closed for changed evidence-bearing requirements that lack positive coverage, negative coverage, or an accepted exemption.

#### Scenario: Missing negative coverage denies
- GIVEN a changed evidence-bearing testing-harness requirement with positive coverage only
- WHEN the matrix gate evaluates the checked-in matrix
- THEN the gate denies the matrix with a missing-negative diagnostic for that requirement

### Requirement: Matrix entries are receipt-backed or explicitly scoped
r[molten.testing.evidence_matrix.receipt_backed_entries] Matrix entries SHOULD bind canonical receipt refs or deterministic commands and MUST reject stale requirement ids, duplicate entries, missing artifact refs, and unsupported coverage kinds.

#### Scenario: Stale requirement id fails closed
- GIVEN a matrix entry naming a requirement id that is absent from accepted specs and active changes
- WHEN the matrix gate validates the entry
- THEN the gate denies the matrix with a stale-reference diagnostic

### Requirement: Matrix exemptions are explicit and diagnostic-only
r[molten.testing.evidence_matrix.exemptions] Coverage exemptions MUST carry a reason class, evidence path or receipt ref, scope, and review note, and MUST NOT satisfy pass evidence for behavioral requirements unless policy explicitly allows it.

#### Scenario: Documentation-only exemption is visible
- GIVEN a documentation-only testing-harness requirement with no executable coverage
- WHEN the matrix includes an exemption for that requirement
- THEN the matrix records the exemption reason and evidence path without treating it as behavioral pass evidence

### Requirement: ALPN registry fixtures cover valid and invalid routing records
r[molten.testing.iroh_alpn_registry_negative_fixtures] Molten SHOULD include positive fixtures for valid registry admission, handler install, replacement, and removal, plus negative fixtures for duplicate ALPN bytes, malformed encoding, wrong owner namespace, stale generation, unsupported lifecycle state, handler-profile mismatch, unsupported incoming ALPN, and attempts to use ALPN routing evidence as authority.

#### Scenario: Duplicate ALPN fixture denies
- GIVEN a fixture with two active registry entries using the same ALPN bytes
- WHEN registry validation runs
- THEN validation denies with a duplicate-ALPN diagnostic and produces no admitted live router map.

#### Scenario: ALPN-as-authority fixture denies
- GIVEN a fixture where a peer has a valid ALPN route and framed-stream receipt but lacks operation authority evidence
- WHEN downstream operation admission runs
- THEN admission denies before side effects and the fixture records ALPN routing as transport evidence only.

#### Scenario: Stale generation fixture preserves live map
- GIVEN a replacement fixture references an old router generation
- WHEN router admission evaluates the replacement
- THEN the decision denies and the expected live advertised ALPN map remains unchanged.

### Requirement: Replay coverage matrix summarizes subsystem readiness
r[molten.determinism.replay_coverage.matrix] Molten SHOULD emit a canonical replay coverage matrix that records subsystem, workflow, replay eligibility, positive replay evidence refs, negative evidence refs, replay index refs when available, and caveat refs.

#### Scenario: Complete matrix passes readiness
- GIVEN every required replay coverage row has replay eligibility, positive replay evidence, negative evidence, and valid refs
- WHEN the replay coverage matrix is generated
- THEN the matrix decision is `pass`
- AND the matrix records each subsystem/workflow exactly once.

#### Scenario: Missing negative evidence denies readiness
- GIVEN a replay coverage row has positive replay evidence but no required negative tamper or exclusion evidence
- WHEN the replay coverage matrix is generated
- THEN the matrix decision is `deny`
- AND diagnostics identify the subsystem and missing evidence class.

### Requirement: Replay smoke suites cover representative subsystems
r[molten.determinism.replay_coverage.subsystem_smoke] Molten SHOULD provide replay smoke evidence for representative harness, node-control, job worker, coordination, remote dataspace, vat, retention, and dogfood release workflows.

#### Scenario: Node-control workflow has replay row
- GIVEN a node-control workflow bundle replay smoke case emits replay verification evidence
- WHEN the coverage matrix is generated
- THEN the node-control row records the workflow, replay verify ref, negative evidence ref, and any caveats.

#### Scenario: Diagnostic-only subsystem is excluded from pass evidence
- GIVEN a subsystem emits live-only or diagnostic-only evidence without deterministic replay support
- WHEN the coverage matrix is generated
- THEN the subsystem row records diagnostic-only or non-replayable eligibility
- AND the row cannot satisfy deterministic replay readiness.

### Requirement: Replay readiness summaries remain evidence-only
r[molten.determinism.replay_coverage.release_readiness_summary] Replay readiness summaries MUST NOT replace individual replay verification, replay rollup, replay index, subsystem gate, source-gate, policy, provenance, authority, transport, resource, release, or retention evidence.

#### Scenario: Summary alone cannot satisfy gate
- GIVEN a replay coverage matrix with a passing summary
- WHEN a gate requires a replay verification receipt for a specific subsystem run
- THEN the summary alone is insufficient
- AND the gate still requires the referenced replay receipt or subsystem gate evidence.

### Requirement: Non-replayable evidence is explicitly classified
r[molten.determinism.replay_coverage.non_replayable_exclusions] Replay coverage rows MUST classify exploratory, live-only, or ambient-state-dependent runs as diagnostic-only or non-replayable and exclude them from deterministic readiness counts.

#### Scenario: Exploratory pass is excluded
- GIVEN an exploratory run has rendered status `pass` but lacks deterministic replay evidence
- WHEN coverage readiness is computed
- THEN the run is classified as non-replayable or diagnostic-only
- AND it is not counted as positive deterministic replay evidence.

### Requirement: Replay coverage behavior is tested
r[molten.determinism.replay_coverage.tests] Molten SHOULD test complete coverage, missing positive evidence, missing negative evidence, duplicate rows, stale refs, diagnostic-only exclusion, and catalog/readiness readback behavior.

#### Scenario: Stale matrix ref denies
- GIVEN a replay coverage row references evidence whose supplied value hashes to a different ref
- WHEN matrix validation runs
- THEN readiness denies
- AND diagnostics include expected and actual refs.

### Requirement: Effect log sequences are complete and monotonic
r[molten.determinism.effect_log.sequence] Replay effect logs MUST have deterministic sequence metadata with no gaps, duplicates, or reordering relative to the consumed replay effect boundaries.

#### Scenario: Ordered complete log passes
- GIVEN a replay report whose effect log entries are monotonic and match every consumed effect boundary
- WHEN effect-log validation runs
- THEN validation passes
- AND replay may continue to compare downstream trace and state refs.

#### Scenario: Missing sequence denies
- GIVEN a replay report whose consumed effect boundaries require a sequence missing from the effect log
- WHEN effect-log validation runs
- THEN validation denies before replay pass evidence
- AND diagnostics identify the missing sequence or boundary ref.

#### Scenario: Duplicate sequence denies
- GIVEN a replay effect log with two entries for the same sequence or request ref
- WHEN effect-log validation runs
- THEN validation denies before any duplicate response is consumed
- AND diagnostics identify the duplicate entry refs.

### Requirement: Effect requests and responses are directly bound
r[molten.determinism.effect_log.request_response_binding] Replay effect logs MUST bind each recorded response to the exact effect request ref, effect kind, turn or boundary ref, and consumed replay observation that used it.

#### Scenario: Response for another request denies
- GIVEN a replay effect log entry whose response ref was recorded for a different request ref
- WHEN replay consumes that entry
- THEN validation denies with a request/response binding diagnostic
- AND no live effect is issued to repair the mismatch.

#### Scenario: Extra unused response denies
- GIVEN a replay effect log with an entry that is not consumed by the replay trace
- WHEN effect-log validation completes
- THEN validation denies with an unused recorded response diagnostic
- AND the extra entry cannot satisfy pass evidence.

### Requirement: Effect logs are handler-profile and run-identity bound
r[molten.determinism.effect_log.handler_profile_binding] Replay effect logs MUST bind the run identity ref and handler profile ref used to record the effects, and replay MUST deny stale logs from different identities or profiles.

#### Scenario: Handler profile mismatch denies
- GIVEN a replay report whose effect log was recorded under a different handler profile ref
- WHEN replay validation evaluates the log
- THEN validation denies before effect consumption
- AND diagnostics include the expected and actual handler profile refs.

#### Scenario: Run identity mismatch denies
- GIVEN a replay report whose effect log belongs to a different artifact, dependency closure, initial state, policy, schema, capability, or seed identity
- WHEN replay validation evaluates the log
- THEN validation denies before treating the log as deterministic evidence
- AND diagnostics bind the stale run identity ref.

### Requirement: Replay denies live effect fallback
r[molten.determinism.effect_log.live_effect_denial] Replay MUST deny any attempt to satisfy a missing or invalid recorded effect by issuing a live external effect, and the denial MUST be represented as canonical failure or replay verification evidence.

#### Scenario: Missing response cannot call live adapter
- GIVEN a replay run reaches an effect boundary with no valid recorded response
- WHEN replay evaluates the boundary
- THEN replay denies as recorded-effects-only
- AND no external adapter request is issued.

### Requirement: Effect-log hardening is tested
r[molten.determinism.effect_log.tests] Molten SHOULD test valid logs plus missing, extra, duplicated, reordered, request/response-mismatched, profile-mismatched, run-identity-mismatched, wrong-effect-kind, and live-effect fallback denial cases.

#### Scenario: Negative matrix denies before final state drift
- GIVEN malformed effect-log fixtures covering every supported denial kind
- WHEN replay tests evaluate them
- THEN each case denies with the expected effect-log diagnostic
- AND final-state drift is not reported as the first divergence when the effect-log error is earlier.

### Requirement: Replay evidence binds deterministic run identity
r[molten.determinism.replay_freshness.identity_binding] Replay verification receipts and replay indexes SHOULD bind the deterministic run identity they verify, including artifact ref, dependency closure ref, initial state ref, schema refs, policy refs, capability refs, handler profile ref, seed or effect-log ref, runtime/tool refs, and replay profile.

#### Scenario: Matching identity is accepted
- GIVEN replay verification evidence whose run identity matches the expected subsystem or release subject identity
- WHEN replay freshness validation runs
- THEN the freshness decision is `pass`
- AND the receipt records the matching run identity ref.

#### Scenario: Changed policy ref denies freshness
- GIVEN replay evidence recorded with a different policy ref than the expected subject identity
- WHEN freshness validation runs
- THEN the freshness decision is `deny`
- AND diagnostics identify the stale policy component.

### Requirement: Replay indexes preserve member identity bindings
r[molten.determinism.replay_freshness.index_binding] Replay indexes SHOULD preserve and summarize run identity refs from their member replay verification receipts, and MUST deny when a member's declared identity ref is malformed or stale for an expected subject.

#### Scenario: Index lists identity refs
- GIVEN a replay index built from identity-bound replay verification receipts
- WHEN the index is emitted
- THEN it records the unique run identity refs represented by the member receipts
- AND each identity ref is content-ref validated.

#### Scenario: Stale member denies index freshness
- GIVEN a replay index with one member receipt whose run identity differs from the expected subject identity
- WHEN index freshness validation runs
- THEN validation denies
- AND diagnostics identify the stale member receipt ref and mismatched identity component.

### Requirement: Replay freshness behavior is tested
r[molten.determinism.replay_freshness.tests] Molten SHOULD test matching identity acceptance and stale artifact, dependency closure, initial state, schema, policy, capability, handler profile, seed/effect-log, runtime, tool, and replay profile denial cases.

#### Scenario: Identity denial matrix identifies components
- GIVEN replay fixtures that each alter one deterministic identity component
- WHEN freshness validation evaluates them
- THEN each case denies with the expected stale component diagnostic
- AND none of the stale receipts can satisfy release-bound replay evidence.

### Requirement: Multi-turn replay comparison is canonical
r[molten.determinism.multiturn_replay.core] Molten MUST provide a deterministic replay comparison core that compares run identity, ordered turn journal refs, ordered effect request and response refs, output refs, and final-state refs by canonical content refs rather than rendered logs.

#### Scenario: Matching multi-turn replay passes
- GIVEN expected and actual replay summaries with the same run identity, turn journal refs, effect log refs, output refs, and final-state refs
- WHEN multi-turn replay comparison runs
- THEN the replay decision is `pass`
- AND the emitted replay receipt binds the compared summary refs.

#### Scenario: Changed turn ref denies
- GIVEN expected and actual replay summaries with matching run identity but a different turn journal ref at one position
- WHEN multi-turn replay comparison runs
- THEN the replay decision is `deny`
- AND the first-divergence evidence identifies the divergent turn position before downstream final-state drift.

### Requirement: First-divergence records include path metadata
r[molten.determinism.multiturn_replay.first_divergence_path] First-divergence records MUST bind divergence kind, turn index, event index when available, boundary kind, actor/session/vat identifier when present, field path, handler profile ref, expected ref, actual ref, and redaction status.

#### Scenario: Effect response divergence names boundary path
- GIVEN a replay whose first mismatch is an effect response boundary in a later turn
- WHEN replay comparison denies
- THEN first-divergence evidence records the effect-response boundary kind, turn index, event index, handler profile ref, expected response ref, actual response ref, and safe redaction status.

#### Scenario: Raw payload is not exposed by default
- GIVEN a first-divergence record for a sensitive trace boundary
- WHEN the record is rendered without trace privacy authority
- THEN the rendered diagnostic shows safe refs and path metadata only
- AND raw payload materialization requires separate privacy evidence.

### Requirement: Replay explain emits canonical evidence
r[molten.determinism.multiturn_replay.explain_cli] The replay explain CLI MUST emit canonical explain evidence for replay comparisons or deny receipts before rendering human-readable summaries.

#### Scenario: Explain summarizes deny receipt
- GIVEN a replay deny receipt with first-divergence evidence
- WHEN `molten test replay explain` is run
- THEN the command emits an explain receipt bound to the deny receipt and first-divergence ref
- AND the rendered summary is diagnostic-only.

#### Scenario: Explain rejects malformed replay evidence
- GIVEN malformed or stale replay evidence
- WHEN `molten test replay explain` is run
- THEN the command fails closed with canonical failure evidence
- AND no rendered summary is accepted as replay verification.

### Requirement: Large replay traces support prefix comparison
r[molten.determinism.multiturn_replay.merkle_prefix] Molten SHOULD compare manifest-backed large replay traces by summary roots and narrowed turn or boundary refs before materializing full trace contents.

#### Scenario: Prefix comparison narrows divergent turn
- GIVEN two large replay trace manifests whose summary roots differ
- WHEN prefix comparison runs
- THEN the comparator identifies the first divergent turn or boundary ref
- AND any partial fetch is covered by chunk range receipt evidence.

#### Scenario: Tampered manifest denies before comparison
- GIVEN a replay trace manifest whose stored bytes do not match its declared content ref
- WHEN prefix comparison attempts to read it
- THEN comparison denies before using the tampered bytes
- AND diagnostics bind the failed manifest ref.

### Requirement: Multi-turn replay behavior is tested
r[molten.determinism.multiturn_replay.tests] Molten SHOULD test positive multi-turn replay stability, negative first-divergence path diagnostics, explain CLI receipts, manifest-backed prefix comparison, and redaction-safe rendering.

#### Scenario: Test matrix covers semantic boundaries
- GIVEN a multi-turn replay fixture with tamper variants for scheduler, input, effect request, effect response, policy decision, hostcall decision, actor output, receipt, output, and state refs
- WHEN replay tests verify each tamper variant
- THEN each case denies with the expected first-divergence kind and path metadata.
