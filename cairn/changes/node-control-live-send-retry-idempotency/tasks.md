# Tasks: Node Control Live Send Retry and Idempotency UX

- [x] [serial] r[molten.node_control_live_send_retry_idempotency.spec.operation_id_guard] Add an optional live-send operation-id guard that fails closed before transport when the derived operation ref differs.
- [x] [serial] r[molten.node_control_live_send_retry_idempotency.spec.retry_receipts] Emit canonical retry receipts for bounded failed live-send join/publish attempts.
- [x] [serial] r[molten.node_control_live_send_retry_idempotency.spec.duplicate_receipts] Emit canonical duplicate-send receipts and suppress re-broadcast when a prior pass send receipt already exists for the derived envelope.
- [x] [parallel] r[molten.node_control_live_send_retry_idempotency.spec.fail_closed_diagnostics] Preserve fail-closed diagnostics for missing/unsupported ticket addresses, operation mismatches, join failures, and duplicate suppression.
- [x] [parallel] r[molten.node_control_live_send_retry_idempotency.spec.transport_non_authority] Keep retry and duplicate receipts out of authority/bootstrap/provenance admission.
- [x] [serial] r[molten.node_control_live_send_retry_idempotency.spec.cli_ux] Expose CLI flags for operation-id guard, bounded attempts, retry receipt export, and duplicate receipt export.
- [x] [parallel] r[molten.node_control_live_send_retry_idempotency.spec.tests] Cover duplicate suppression and operation-id mismatch paths in tests.
