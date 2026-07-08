# Tasks: preserves-boundary-field-contracts

## Phase 1: Field contract model

- [x] [serial] r[molten.preserves_boundary_field_contracts.field_contracts] Add reusable boundary field contracts for non-empty strings, enums, stable ids, bounded sequences, ref sets, unique ref sets, and typed embedded records.
- [x] [parallel] r[molten.preserves_boundary_field_contracts.field_contract_denials] Add helper-level positive and negative tests for every new field contract.

## Phase 2: High-risk boundary tightening

- [x] [serial] r[molten.preserves_boundary_field_contracts.high_risk_tightening] Replace broad field kinds in node-control ingress, plugin extension contracts, retention receipts, chain bundles, and release bundles where domain rules are known.
- [x] [parallel] r[molten.preserves_boundary_field_contracts.semantic_boundary] Confirm narrowed shape validation does not replace authority, policy, provenance, resource, replay, transport, or execution gates.

## Phase 3: Validation

- [x] [parallel] r[molten.preserves_boundary_field_contracts.field_contract_denials] Add negative fixtures for invalid decisions, empty required ref sets, duplicate unique refs, oversized sequences, and unsupported embedded records.
- [x] [serial] r[molten.preserves_boundary_field_contracts.high_risk_tightening] Run focused boundary/parser tests and `nix run path:$PWD#cairn -- validate --root .`.
