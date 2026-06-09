# Testing Harness Delta: sealed repro verify/unpack

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
