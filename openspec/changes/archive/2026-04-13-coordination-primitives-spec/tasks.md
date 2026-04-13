## Phase 1: Scope and overlap

- [x] Review the current `coordination` and `distributed-lockset` specs together and confirm the umbrella/domain split stays non-duplicative.
- [x] Decide whether `DistributedWorkerCoordinator` remains under the umbrella coordination domain or should move to its own spec later.

## Phase 2: Coordination spec expansion

- [x] Extend `openspec/specs/coordination/spec.md` with requirements for leader election, counters, sequences, rate limiting, barriers, semaphores, read-write locks, queues, service registry, and worker coordination.
- [x] Preserve `openspec/specs/distributed-lockset/spec.md` as the detailed lock-set contract instead of copying those rules into `coordination`.
- [x] Add a coordination RPC requirement that covers native and plugin-backed implementations under one request/response contract.

## Phase 3: Validation

- [x] Reconcile the new coordination requirements against `crates/aspen-coordination/`, `crates/aspen-client-api/`, and the existing coordination test suite.
- [x] Run `openspec validate coordination-primitives-spec` before archive and check that the delta headers merge cleanly into the current main `coordination` spec.
