## Phase 1: Dataspace and turn predicates

- [x] [serial] r[molten.trellis_runtime.assertion_visibility] Add Trellis-backed predicates for dataspace assertion ownership, deduplication, visibility, and automatic retraction.
- [x] [serial] r[molten.trellis_runtime.turn_commit_rollback] Add Trellis-backed predicates for pending-action invisibility, atomic turn commit, and rollback on failure or denial.
- [x] [serial] r[molten.trellis_runtime.preserves_pattern_subset] Define a bounded Trellis-friendly Preserves pattern/value subset with deterministic matching and binding order.
- [x] [parallel] r[molten.trellis_runtime.observe_delivery] Add Trellis-backed predicates for Observe delivery of current/future assertions and matching retraction propagation.

## Phase 2: Object/vat predicates

- [x] [serial] r[molten.trellis_runtime.promise_state] Add Trellis-backed promise/vow state-machine predicates for pending, resolved, broken, cancelled, timed-out, and causal failure propagation states.
- [x] [serial] r[molten.trellis_runtime.promise_pipeline] Add Trellis-backed bounded promise-pipelining predicates for queue bounds, forwarding order, and failure cleanup.
- [x] [serial] r[molten.trellis_runtime.revocation_cleanup] Add Trellis-backed predicates for revoked references denying future use and cleaning dependent assertions, subscriptions, pending calls, and child references.
- [x] [parallel] r[molten.trellis_runtime.actormap_transaction] Add Trellis-backed predicates for actormap delta commit/rollback, spawned object visibility, and removed object invalidation.
- [x] [parallel] r[molten.trellis_runtime.near_far_refs] Add Trellis-backed predicates admitting synchronous calls only for live same-vat near references and requiring asynchronous semantics for far references.

## Phase 3: Persistence and service predicates

- [x] [serial] r[molten.trellis_runtime.snapshot_authority] Add Trellis-backed predicates ensuring snapshot authority claims are subsets of held or explicitly admitted authority.
- [x] [parallel] r[molten.trellis_runtime.service_dependencies] Add Trellis-backed service dependency predicates for demand, readiness, failure, force-run, restart, reverse dependency, and shutdown admission.
- [x] [parallel] r[molten.trellis_runtime.predicate_receipts] Define receipt/evidence names for runtime predicate applications so Cairn receipts can identify the applied predicate and decision.

## Phase 4: Tests and integration

- [x] [serial] r[molten.trellis_runtime.integration_tests] Add integration tests showing Molten runtime admission calls the Trellis-backed predicates for assertion visibility, turn commit/rollback, patterns, promises, and revocation.
- [ ] [parallel] r[molten.trellis_runtime.property_tests] Add Hegel property tests over bounded models for assertion owners, turn deltas, pattern matches, promise pipelines, revocation graphs, snapshots, and service dependencies.
