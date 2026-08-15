# Tasks: canonical-ci-test-run-receipt

## Receipt schema

- [x] [serial] r[molten.testing.ci_run_receipt.canonical_receipt] Define a canonical CI test-run receipt schema that binds source ref, profile id, command surface, nextest config ref, Cargo metadata ref, binaries metadata ref, rendered JUnit ref, counts, decision, diagnostics, and caveats.
- [x] [parallel] r[molten.testing.ci_run_receipt.junit_view_only] Document that JUnit is a rendered view and cannot replace the canonical CI receipt.

## Nix and CLI wiring

- [x] [serial] r[molten.testing.ci_run_receipt.nix_nextest_binding] Wire the Nix nextest check or an adjacent CLI command to emit the canonical CI test-run receipt beside existing metadata and JUnit outputs.
- [x] [parallel] r[molten.testing.ci_run_receipt.deny_on_missing_metadata] Fail closed when Cargo metadata, binaries metadata, nextest config, JUnit output, profile id, or counts are missing, stale, or mismatched.

## Tests and validation

- [x] [parallel] r[molten.testing.ci_run_receipt.canonical_receipt] Add positive receipt parser/summary tests and negative tests for missing metadata, stale profile id, mismatched counts, and JUnit-only evidence.
- [x] [serial] r[molten.testing.ci_run_receipt.nix_nextest_binding] Update README release/readback docs to include the CI receipt path and refs.
- [x] [serial] r[molten.testing.ci_run_receipt.deny_on_missing_metadata] Run focused CI receipt tests, nextest config validation, and Cairn validation; record any Nix builder blockers.
