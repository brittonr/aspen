# Tasks: Restore the Tracey requirement root

## Phase 1: Implementation

- [x] [serial] Point the guard and classifier at `.cairn/specs` and fail closed on a missing or empty root, with a missing-root negative self-test. r[aspen.cas.verification]
- [x] [serial] Regenerate the classification inventory, summary, digests, and path metadata. r[aspen.cas.verification]
- [x] [serial] Add evidence markers only where code, tests, documents, or checks implement or verify the uncovered requirements. r[aspen.cas.verification]
- [x] [serial] Restate the four CAS requirements with standalone markers in the MODIFIED delta. r[aspen.cas.contract] r[aspen.cas.decision] r[aspen.cas.boundary] r[aspen.cas.verification]
- [x] [serial] Move the traceability scan roots, gate fixture, vendored policy roots, and README text to the `.cairn/` layout. r[aspen.cas.verification]

## Phase 2: Validation

- [x] [serial] Run the guard and classifier self-tests and record the guard counts before and after. r[aspen.cas.verification]
- [x] [serial] Build `inherited-tracey-debt`, `requirement-traceability-gate`, and `contract-export-drift-gate`, and record the results and remaining residue in `evidence/verification.md`. r[aspen.cas.verification]
