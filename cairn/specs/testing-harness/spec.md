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
