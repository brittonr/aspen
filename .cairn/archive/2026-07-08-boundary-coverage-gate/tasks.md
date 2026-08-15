# Tasks: boundary-coverage-gate

## Coverage model

- [x] [serial] r[molten.testing.boundary_coverage.gate] Define a boundary coverage gate model over typed harness reports and traceability entries.
- [x] [parallel] r[molten.testing.boundary_coverage.positive_negative] Define required positive and negative boundary classes for evidence-bearing testing-harness requirements.

## Report and gate integration

- [x] [serial] r[molten.testing.boundary_coverage.gate] Add report or receipt output that lists observed boundaries, missing boundaries, requirement ids, evidence refs, and gate decision.
- [x] [parallel] r[molten.testing.boundary_coverage.exemptions] Add explicit exemption support with reason class, evidence path, scope, and diagnostic-only caveat.

## Tests and validation

- [x] [parallel] r[molten.testing.boundary_coverage.positive_negative] Add positive tests for complete boundary coverage and negative tests for missing denial path, missing pass path, stale evidence ref, unsupported boundary class, and exemption without evidence.
- [x] [serial] r[molten.testing.boundary_coverage.gate] Update docs with boundary classes and local commands.
- [x] [serial] r[molten.testing.boundary_coverage.exemptions] Run focused boundary coverage tests and Cairn validation; record any deferred boundary classes.
