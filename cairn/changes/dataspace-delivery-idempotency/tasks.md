## Phase 1: Idempotency records

- [x] [serial] r[molten.dataspace_delivery_idempotency.spec.operation_identity] Define `operation-id-v1` with scope, producer, consumer, sequence, intent, payload, policy, and checks.
- [x] [serial] r[molten.dataspace_delivery_idempotency.spec.before_commit] Define delivery window, dedup entry, idempotency receipt, and retry receipt DTOs.
- [x] [parallel] r[molten.dataspace_delivery_idempotency.spec.operation_identity] Define scope profiles for actor turn, service lifecycle, protocol session, remote topic, job worker, and control command.
- [x] [parallel] r[molten.dataspace_delivery_idempotency.spec.retention] Classify operation/dedup/idempotency artifacts in ledger/catalog views.

## Phase 2: Dedup store and runtime gates

- [x] [serial] r[molten.dataspace_delivery_idempotency.spec.retention] Implement Redb-backed dedup windows with rebuildable ledger refs and retention pins.
- [x] [serial] r[molten.dataspace_delivery_idempotency.spec.before_commit] Check dedup/sequence windows before local or remote dataspace side effects commit.
- [x] [parallel] r[molten.dataspace_delivery_idempotency.spec.before_commit] Suppress duplicate side effects while returning the prior semantic result/evidence ref.
- [x] [parallel] r[molten.dataspace_delivery_idempotency.spec.operation_identity] Deny conflicting duplicates, stale sequences, and invalid gaps with canonical diagnostics.

## Phase 3: Surface integration

- [x] [serial] r[molten.dataspace_delivery_idempotency.spec.operation_identity] Bind operation refs into remote dataspace envelopes and transport/admission receipts.
- [x] [serial] r[molten.dataspace_delivery_idempotency.spec.operation_identity] Bind operation refs into protocol messages and endpoint state transitions.
- [x] [parallel] r[molten.dataspace_delivery_idempotency.spec.before_commit] Bind operation refs into service lifecycle events and job worker requests/results.
- [x] [parallel] r[molten.dataspace_delivery_idempotency.spec.before_commit] Include idempotency receipts in deterministic replay logs and first-divergence diagnostics.

## Phase 4: Tests

- [x] [serial] r[molten.dataspace_delivery_idempotency.spec.before_commit] Test duplicate remote assertion/message delivery suppresses second commit and returns prior result evidence.
- [x] [serial] r[molten.dataspace_delivery_idempotency.spec.operation_identity] Test same operation id with changed payload/evidence denies before side effects.
- [x] [parallel] r[molten.dataspace_delivery_idempotency.spec.before_commit] Test stale, gap, retry, and reconnect scenarios.
- [x] [parallel] r[molten.dataspace_delivery_idempotency.spec.operation_identity] Add Hegel properties for operation identity determinism, duplicate suppression, and no-global-sequence invariant.
