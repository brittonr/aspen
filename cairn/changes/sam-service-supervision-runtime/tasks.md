## Reconciliation

This umbrella change was split into the smaller drained changes `sam-service-records-ledger`, `sam-service-demand-runtime`, and `sam-service-supervision-cleanup`. The implementation landed in `src/service_records.rs`, `src/service_runtime.rs`, and `src/service_supervision.rs`; the split changes hold the detailed task evidence.

- [x] [serial] r[molten.sam_service_supervision.spec.demand_start] Service manifests, demand assertions, startup admission, dependency readiness, lifecycle receipts, replay identity, and CLI/runtime tests are implemented by `sam-service-records-ledger` and `sam-service-demand-runtime`.
- [x] [serial] r[molten.sam_service_supervision.spec.supervision] Logical links, monitors, deterministic restart budgets, monitor notification receipts, replay validation, and supervision tests are implemented by `sam-service-supervision-cleanup`.
- [x] [serial] r[molten.sam_service_supervision.spec.cleanup] Authority-bound cleanup, owned-state retraction, foreign-state denial, retention-bound cleanup receipts, and revocation tests are implemented by `sam-service-supervision-cleanup`.
