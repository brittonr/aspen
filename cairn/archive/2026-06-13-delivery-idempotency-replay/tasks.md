## Phase 1: Operation identity

- [x] [serial] r[molten.delivery.operation_identity] Define canonical operation ids with scope ref, producer, consumer, sequence, intent/effect kind, payload or request ref, and policy refs while keeping authority/capability evidence separate.
- [x] [serial] r[molten.delivery.classes] Define implemented delivery evidence outcomes: first, duplicate, conflict, stale, gap, retry, and one-shot disclosure boundaries.
- [x] [parallel] r[molten.delivery.no_exact_once_claim] Document that Molten does not claim network-level exactly-once delivery or timeout-as-non-execution proof.
- [x] [parallel] r[molten.delivery.receipt_model] Emit canonical operation-id, window, dedup-entry, idempotency, and retry receipts.

## Phase 2: Dedup and replay bounds

- [x] [serial] r[molten.delivery.dedup_ledger] Add local Redb-backed dedup windows with operation id, request hash/payload ref, response/semantic result ref, receipt refs, retention refs, and scope.
- [x] [serial] r[molten.delivery.conflict_detection] Reject duplicate operation ids or scoped sequences with conflicting payload or evidence hashes.
- [x] [serial] r[molten.delivery.sequence_windows] Enforce bounded sequence/replay windows per session/sender scope and avoid global sequence coupling.
- [x] [parallel] r[molten.delivery.retry_schedule] Make retry guidance deterministic under logical sequence windows and keep timeout/non-execution claims evidence-only.

## Phase 3: Integration boundaries

- [x] [serial] r[molten.delivery.dataspace_effects] Apply idempotency keys to remote dataspace and node-control ingress deliveries before local side effects or durable enqueue.
- [x] [serial] r[molten.delivery.storage_mutations] Define the fail-closed boundary that typed storage writes and upgrade migrations must carry explicit matching operation-id/idempotency evidence before claiming delivery dedup.
- [x] [parallel] r[molten.delivery.choreography] Define the evidence-only boundary for future protocol/session/op-index idempotency without replacing protocol authority or transport admission.
- [x] [parallel] r[molten.delivery.remote_jobs_upgrades] Bind operation ids where present for remote ingress, control-plane commands, and job worker paths, while keeping remote sync/upgrades as future explicit extensions.

## Phase 4: Tests

- [x] [serial] r[molten.delivery.duplicate_tests] Add tests that duplicate operation ids return prior receipts or semantic result refs and reject conflicts.
- [x] [serial] r[molten.delivery.replay_window_tests] Add tests for stale, future-gap/retry, duplicate sequence, and independent-scope behavior.
- [x] [parallel] r[molten.delivery.timeout_tests] Cover timeout semantics as evidence-only documentation: retries/timeouts do not prove remote non-execution or grant authority.
- [x] [parallel] r[molten.delivery.property_tests] Add Hegel-style property tests for operation identity determinism, dedup ledger invariants, independent scopes, and idempotent replay behavior.
