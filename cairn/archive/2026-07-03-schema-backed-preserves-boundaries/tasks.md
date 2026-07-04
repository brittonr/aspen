# Tasks: schema-backed-preserves-boundaries

## Phase 1: Schema adapter

- [x] [serial] r[molten.preserves_schema_boundaries.schema_adapter] Add a pure `preserves-schema` validation adapter in `preserves_rail`.
- [x] [parallel] r[molten.preserves_schema_boundaries.schema_artifacts] Check in versioned schema artifacts for the first boundary family allowlist.
- [x] [parallel] r[molten.preserves_schema_boundaries.schema_artifacts] Bind schema artifact refs in test fixtures and release evidence diagnostics.

## Phase 2: Boundary adoption

- [x] [serial] r[molten.preserves_schema_boundaries.high_risk_records] Require schema validation before node-control ingress acceptance.
- [x] [serial] r[molten.preserves_schema_boundaries.high_risk_records] Require schema validation before plugin hostcall and extension-contract acceptance.
- [x] [serial] r[molten.preserves_schema_boundaries.high_risk_records] Require schema validation before evidence-chain bundle, retention receipt, and release bundle acceptance.

## Phase 3: Tests and validation

- [x] [parallel] r[molten.preserves_schema_boundaries.schema_denials] Add positive valid-fixture tests for each adopted boundary family.
- [x] [parallel] r[molten.preserves_schema_boundaries.schema_denials] Add negative tests for wrong label, missing field, wrong type, malformed checks, unsupported version, and extra critical fields.
- [x] [serial] r[molten.preserves_schema_boundaries.schema_adapter] r[molten.preserves_schema_boundaries.high_risk_records] Run focused schema, node, plugin, evidence, retention, and dogfood tests.
