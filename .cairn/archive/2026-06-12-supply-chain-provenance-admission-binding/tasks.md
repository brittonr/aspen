## Phase 1: Admission binding

- [x] [serial] r[molten.provenance_admission_binding.spec.build_record_binding] Add explicit build record refs to provenance records for reproducible trust claims.
- [x] [serial] r[molten.provenance_admission_binding.spec.verify_receipt_required] Require passing build verification receipts before admitting `reproducible-verified` provenance.
- [x] [parallel] r[molten.provenance_admission_binding.spec.evidence_only] Preserve the evidence-only boundary for build verification receipts.

## Phase 2: CLI and node-control integration

- [x] [serial] r[molten.provenance_admission_binding.spec.cli_evaluate_build_evidence] Extend provenance evaluation CLI with explicit build verification receipt inputs.
- [x] [serial] r[molten.provenance_admission_binding.spec.node_control_binding] Make node-control install/run provenance gates validate build verification binding before side effects.
- [x] [parallel] r[molten.provenance_admission_binding.spec.receipt_refs] Bind considered build verification receipt refs into provenance evaluation receipts.

## Phase 3: Tests

- [x] [serial] r[molten.provenance_admission_binding.spec.matching_pass] Add tests for matching reproducible build evidence admission.
- [x] [serial] r[molten.provenance_admission_binding.spec.missing_or_mismatch_deny] Add tests for missing, denied, mismatched, and unbound build verification evidence.
