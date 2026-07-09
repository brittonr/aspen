# Tasks: test-evidence-matrix

## Matrix schema

- [x] [serial] r[molten.testing.evidence_matrix.checked_in_manifest] Define the checked-in matrix schema in Nickel or canonical Preserves, with typed fields for requirement id, coverage kind, target, command, artifact refs, evidence scope, and caveats.
- [x] [parallel] r[molten.testing.evidence_matrix.exemptions] Define explicit exemption entries with reason class, evidence path, expiry or review notes, and diagnostic-only scope.

## Gate behavior

- [x] [serial] r[molten.testing.evidence_matrix.changed_requirement_gate] Implement pure validation that fails changed evidence-bearing requirements lacking positive or negative coverage unless an accepted exemption is present.
- [x] [parallel] r[molten.testing.evidence_matrix.receipt_backed_entries] Bind matrix validation to canonical traceability receipts and reject stale requirement ids, duplicate entries, missing artifact refs, and unsupported coverage kinds.

## Tests and readback

- [x] [parallel] r[molten.testing.evidence_matrix.changed_requirement_gate] Add positive fixtures for complete coverage and negative fixtures for missing positive coverage, missing negative coverage, stale ids, duplicate entries, and missing artifact refs.
- [x] [serial] r[molten.testing.evidence_matrix.checked_in_manifest] Document the matrix path and local review command in README or testing docs.
- [x] [serial] r[molten.testing.evidence_matrix.receipt_backed_entries] Run focused traceability tests, the matrix gate, and Cairn validation; record blockers if any check is unavailable.
