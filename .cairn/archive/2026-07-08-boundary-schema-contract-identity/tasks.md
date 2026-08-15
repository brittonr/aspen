# Tasks: boundary-schema-contract-identity

## Phase 1: Contract identity core

- [x] [serial] r[molten.boundary_schema_contract_identity.full_contract_ref] Extend boundary schema artifact values to include ordered field labels, field kinds, and declared constraints.
- [x] [parallel] r[molten.boundary_schema_contract_identity.full_contract_ref] Add pure tests proving unchanged specs keep stable refs and same-arity field-kind drift changes refs.
- [x] [parallel] r[molten.boundary_schema_contract_identity.stale_schema_denial] Add stale schema diagnostics for mismatched expected schema refs.

## Phase 2: Boundary adoption

- [x] [serial] r[molten.boundary_schema_contract_identity.receipt_binding] Update schema-backed boundary validation reports to bind the strengthened schema ref.
- [x] [serial] r[molten.boundary_schema_contract_identity.compatibility_note] Record any expected-ref migrations in tests or release-evidence fixtures.

## Phase 3: Validation

- [x] [parallel] r[molten.boundary_schema_contract_identity.stale_schema_denial] Add negative fixtures for field reorder, label drift, and constraint drift.
- [x] [serial] r[molten.boundary_schema_contract_identity.full_contract_ref] r[molten.boundary_schema_contract_identity.receipt_binding] Run focused `preserves_rail` tests and `nix run path:$PWD#cairn -- validate --root .`.
