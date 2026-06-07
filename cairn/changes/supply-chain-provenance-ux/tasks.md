## Phase 1: CLI surface

- [x] [serial] r[molten.provenance_ux.spec.record_fixture_cli] Add provenance fixture and record commands that emit canonical Preserves records.
- [x] [serial] r[molten.provenance_ux.spec.evaluate_receipts] Add provenance evaluation command that writes canonical provenance receipts from explicit provenance files.
- [x] [parallel] r[molten.provenance_ux.spec.evaluate_receipts] Add read-only provenance record/receipt summaries.

## Phase 2: Validation and docs

- [x] [serial] r[molten.provenance_ux.spec.evaluate_receipts] Cover reviewed pass and sandbox-only node-control denial through the CLI helper.
- [x] [parallel] r[molten.provenance_ux.spec.evidence_only] Document that provenance UX receipts are evidence only and do not grant authority, policy, resource, transport, execution, or source-gate trust.
