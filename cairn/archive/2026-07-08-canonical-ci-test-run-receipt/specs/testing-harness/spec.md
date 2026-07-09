# Testing Harness Delta: Canonical CI Test-run Receipt

## ADDED Requirements

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
