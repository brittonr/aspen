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
