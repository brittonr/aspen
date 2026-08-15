## ADDED Requirements

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
