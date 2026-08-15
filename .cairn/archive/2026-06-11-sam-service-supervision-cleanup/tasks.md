## Phase 1: Supervision records

- [x] [serial] r[molten.sam_service_supervision_cleanup.spec.logical_supervision] Define canonical `service-link-v1` and `service-monitor-v1` records with service ids, refs, failure propagation policy, observer refs, and checks.
- [x] [serial] r[molten.sam_service_supervision_cleanup.spec.bounded_restart] Define restart decision receipt fields for attempt counters, logical window, backoff slot, authority/resource refs, diagnostics, and checks.
- [x] [parallel] r[molten.sam_service_supervision_cleanup.spec.owned_cleanup] Extend/validate `service-cleanup-receipt-v1` with owned assertion, observer, live-ref, pending-effect, retraction, revocation, and retention refs.
- [x] [parallel] r[molten.sam_service_supervision_cleanup.spec.logical_supervision] Classify supervision, restart, monitor, and cleanup artifacts in ledger/catalog/MCP views.

## Phase 2: Failure, monitor, and restart runtime

- [x] [serial] r[molten.sam_service_supervision_cleanup.spec.logical_supervision] Commit service failure status assertions and lifecycle receipts in deterministic turns.
- [x] [serial] r[molten.sam_service_supervision_cleanup.spec.logical_supervision] Notify monitors in deterministic service/ref order and bind notification refs into lifecycle receipts.
- [x] [serial] r[molten.sam_service_supervision_cleanup.spec.bounded_restart] Evaluate restart policies with bounded attempts, logical windows, backoff slots, authority state, and resource receipts.
- [x] [parallel] r[molten.sam_service_supervision_cleanup.spec.bounded_restart] Deny restart loops that exceed policy/resource bounds and publish final stopped/failed status.

## Phase 3: Cleanup and revocation

- [x] [serial] r[molten.sam_service_supervision_cleanup.spec.owned_cleanup] Track service-owned assertions, observers, live refs, exposed refs, and pending effect intents for cleanup.
- [x] [serial] r[molten.sam_service_supervision_cleanup.spec.owned_cleanup] Run cleanup on authority revocation and bind revocation refs in cleanup receipts.
- [x] [parallel] r[molten.sam_service_supervision_cleanup.spec.owned_cleanup] Deny cleanup of state whose service ownership cannot be proven.
- [x] [parallel] r[molten.sam_service_supervision_cleanup.spec.cleanup_replay_retention] Bind cleanup receipts as inputs to retention/GC eligibility without bypassing retention policy gates.

## Phase 4: Tests

- [x] [serial] r[molten.sam_service_supervision_cleanup.spec.bounded_restart] Test failure assertion, monitor notification, bounded restart pass, and bounded restart denial.
- [x] [serial] r[molten.sam_service_supervision_cleanup.spec.owned_cleanup] Test authority revocation retracts all owned readiness/live-ref assertions and pending intents.
- [x] [parallel] r[molten.sam_service_supervision_cleanup.spec.cleanup_replay_retention] Test replay divergence for monitor order, restart decision, cleanup set, and resource denial.
- [x] [parallel] r[molten.sam_service_supervision_cleanup.spec.cleanup_replay_retention] Add Hegel properties for cleanup completeness, no-foreign-deletion, restart boundedness, and deterministic monitor order.
