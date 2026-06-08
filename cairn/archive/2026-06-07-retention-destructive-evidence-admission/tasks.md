## Phase 1: Admission model

- [x] [serial] Add typed retention evidence admission receipts and parsing helpers. r[molten.retention.evidence_admission_model]
- [x] [serial] Admit destructive evidence refs against requester/object/class/action scope before apply-mode mutation. r[molten.retention.evidence_scope_binding]

## Phase 2: Subsystem plumbing

- [x] [serial] Gate ledger GC, chunk GC, and eval-cache invalidation on admitted policy, authority, evidence, reference-index, and remote-GC receipts. r[molten.retention.destructive_admission_gate]
- [x] [serial] Surface admission refs and diagnostics in subsystem receipts and CLI flags. r[molten.retention.admission_receipt_diagnostics]

## Phase 3: Verification

- [x] [serial] Add pass/fail tests for forged, stale, mismatched, missing, retained, incomplete-index, and remote-uncertainty evidence. r[molten.retention.admission_tests]
- [x] [serial] Validate, sync, archive, and push the change. r[molten.retention.admission_tests]
