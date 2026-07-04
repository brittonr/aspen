# Tasks: structural-preserves-value-inspection

## Phase 1: Visitor core

- [x] [serial] r[molten.preserves_value_inspection.structural_scan] Add a pure bounded `IOValue` visitor with explicit predicate scopes.
- [x] [parallel] r[molten.preserves_value_inspection.marker_detection] Add marker predicates for secret/confidential/credential/private/encrypted-ref records.
- [x] [parallel] r[molten.preserves_value_inspection.ref_retention] Add structural content-ref detection for cleanup and retention checks.

## Phase 2: Call-site migration

- [x] [serial] r[molten.preserves_value_inspection.marker_detection] Replace service sensitivity text scans with structural marker inspection.
- [x] [serial] r[molten.preserves_value_inspection.ambient_token_denial] Replace job ambient/mobile-token text scans with structural record/symbol inspection.
- [x] [serial] r[molten.preserves_value_inspection.ref_retention] Replace upgrade cleanup retained-ref scans with structural ref traversal.

## Phase 3: Tests and validation

- [x] [parallel] r[molten.preserves_value_inspection.structural_scan] Add positive tests for nested structural markers and refs.
- [x] [parallel] r[molten.preserves_value_inspection.structural_scan] Add negative tests for inert strings that look like rendered records but are not structural markers.
- [x] [serial] r[molten.preserves_value_inspection.marker_detection] r[molten.preserves_value_inspection.ambient_token_denial] r[molten.preserves_value_inspection.ref_retention] Run focused service, job, upgrade, and Preserves tests.
