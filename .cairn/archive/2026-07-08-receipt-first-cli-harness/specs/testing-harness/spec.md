# Testing Harness Delta: Receipt-first CLI Harness

## ADDED Requirements

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
