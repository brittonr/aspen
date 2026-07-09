# Tasks: generated-tamper-negative-matrix

## Matrix core

- [x] [serial] r[molten.testing.tamper_matrix.generated_cases] Define a reusable tamper-case model and pure generator for selected evidence artifact families.
- [x] [parallel] r[molten.testing.tamper_matrix.coverage] Select the initial artifact families: harness reports, gate receipts, repro bundles, redaction evidence, and release evidence bundles.

## Fail-closed behavior

- [x] [serial] r[molten.testing.tamper_matrix.fail_closed] Add parser and gate tests that assert every generated negative case denies before pass evidence and reports the expected diagnostic class.
- [x] [parallel] r[molten.testing.tamper_matrix.generated_cases] Add positive control fixtures for each artifact family so generated negatives are compared against known-valid inputs.

## Traceability and docs

- [x] [parallel] r[molten.testing.tamper_matrix.coverage] Record tamper-matrix entries in the checked-in evidence matrix or traceability coverage.
- [x] [serial] r[molten.testing.tamper_matrix.fail_closed] Document how to add a new artifact family to the matrix.
- [x] [serial] r[molten.testing.tamper_matrix.generated_cases] Run focused tamper-matrix tests and Cairn validation; record any artifact families deferred.
