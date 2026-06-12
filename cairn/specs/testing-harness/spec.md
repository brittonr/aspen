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
