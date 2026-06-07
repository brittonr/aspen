## Phase 1: CLI surface

- [x] [serial] r[molten.delivery_idempotency_ux.spec.cli_scope_operation] Add delivery scope and operation id commands that emit canonical Preserves records.
- [x] [serial] r[molten.delivery_idempotency_ux.spec.cli_check_receipts] Add a delivery check command that writes idempotency receipts from an explicit store root.
- [x] [parallel] r[molten.delivery_idempotency_ux.spec.cli_check_receipts] Add stored receipt lookup and artifact summary commands.

## Phase 2: Validation and docs

- [x] [serial] r[molten.delivery_idempotency_ux.spec.cli_check_receipts] Cover first-delivery and duplicate-suppression decisions through the CLI helper.
- [x] [parallel] r[molten.delivery_idempotency_ux.spec.evidence_only] Document that delivery CLI receipts are evidence only and do not grant authority, provenance, policy, or transport trust.
