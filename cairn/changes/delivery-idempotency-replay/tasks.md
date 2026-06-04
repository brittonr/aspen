## Phase 1: Operation identity

- [ ] [serial] r[molten.delivery.operation_identity] Define canonical operation ids with session id, sequence, cause, target, effect kind, request hash, and policy/capability refs.
- [ ] [serial] r[molten.delivery.classes] Define delivery classes for ephemeral, deduped, transactional, compensating, and one-shot external operations.
- [ ] [parallel] r[molten.delivery.no_exact_once_claim] Document that Molten does not claim network-level exactly-once delivery.
- [ ] [parallel] r[molten.delivery.receipt_model] Emit receipts for attempts, accepts, commits, dedup hits, retries, timeouts, cancellations, and replay rejections.

## Phase 2: Dedup and replay bounds

- [ ] [serial] r[molten.delivery.dedup_ledger] Add local dedup ledgers with operation id, request hash, response hash, receipt refs, expiry, and scope.
- [ ] [serial] r[molten.delivery.conflict_detection] Reject duplicate operation ids with conflicting request hashes.
- [ ] [serial] r[molten.delivery.sequence_windows] Enforce bounded sequence/replay windows per session/sender.
- [ ] [parallel] r[molten.delivery.retry_schedule] Make retry and timeout schedules deterministic under logical time or recorded playback.

## Phase 3: Integration

- [ ] [serial] r[molten.delivery.dataspace_effects] Apply idempotency keys to local dataspace messages and effect requests.
- [ ] [serial] r[molten.delivery.storage_mutations] Apply dedup to typed storage writes and migration tasks.
- [ ] [parallel] r[molten.delivery.choreography] Apply protocol/session/op indices to choreography send/receive/choice delivery.
- [ ] [parallel] r[molten.delivery.remote_jobs_upgrades] Apply operation ids to remote sync, job DAG stages, and upgrade tasks.

## Phase 4: Tests

- [ ] [serial] r[molten.delivery.duplicate_tests] Add tests that duplicate operation ids return prior receipts or reject conflicts.
- [ ] [serial] r[molten.delivery.replay_window_tests] Add tests for stale, future, and duplicate sequence rejection.
- [ ] [parallel] r[molten.delivery.timeout_tests] Add tests showing timeout does not imply remote non-execution.
- [ ] [parallel] r[molten.delivery.property_tests] Add Hegel property tests for dedup ledger invariants and idempotent replay behavior.
