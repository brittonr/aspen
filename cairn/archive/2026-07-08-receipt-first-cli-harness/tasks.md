# Tasks: receipt-first-cli-harness

## CLI assertion model

- [x] [serial] r[molten.testing.cli_receipt_first.normative_artifacts] Define the CLI test convention that canonical Preserves artifacts and receipts are normative for evidence-bearing command assertions.
- [x] [parallel] r[molten.testing.cli_receipt_first.stdout_diagnostic_only] Audit existing CLI harness tests and label stdout/stderr checks as diagnostic-only unless they are paired with parsed artifact assertions.

## Helper refactor

- [x] [serial] r[molten.testing.cli_receipt_first.normative_artifacts] Add or consolidate shared CLI test helpers for reading Preserves files, parsing receipts, asserting decision/kind/refs, and checking diagnostics.
- [x] [parallel] r[molten.testing.cli_receipt_first.negative_fail_closed] Add negative CLI fixtures for malformed report input, stale receipt refs, diagnostic-only bundles, missing metadata, and denied gate outputs.

## Validation

- [x] [parallel] r[molten.testing.cli_receipt_first.negative_fail_closed] Ensure every new CLI test file or slice includes both positive and negative cases.
- [x] [serial] r[molten.testing.cli_receipt_first.stdout_diagnostic_only] Update testing docs to state that rendered CLI text is a view, not normative evidence.
- [x] [serial] r[molten.testing.cli_receipt_first.normative_artifacts] Run focused CLI harness tests and Cairn validation; record any command surfaces deferred for follow-up.
