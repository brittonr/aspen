## Phase 1: Vat/object execution model

- [x] [serial] r[molten.runtime_spine.goblins_reference_boundary] Document Spritely Goblins and OCapN/CapTP as non-normative design references, not implementation or wire-compatibility targets.
- [x] [serial] r[molten.runtime_spine.vat_model] Define Molten vats, near references, far references, and the rule that only near references can be called synchronously.
- [x] [serial] r[molten.runtime_spine.transactional_actormap] Define the transactional actormap model for object state, spawn/remove operations, pending actions, commit, and rollback.
- [x] [parallel] r[molten.runtime_spine.object_capability_refs] Define object references as capability-bearing authority and specify how references cross Preserves envelope boundaries.
- [ ] [parallel] r[molten.runtime_spine.no_ambient_object_authority] Ensure new objects start without ambient filesystem, network, clock, process, dataspace, or host-resource authority unless explicitly endowed.

## Phase 2: Far calls, promises, and reference control

- [ ] [serial] r[molten.runtime_spine.promise_vows] Define promise/vow results for far-object calls, including success, failure, cancellation, timeout, and causal failure propagation.
- [x] [serial] r[molten.runtime_spine.promise_pipelining] Define bounded promise pipelining for queued calls against unresolved future references.
- [x] [serial] r[molten.runtime_spine.revocable_proxies] Define revocable and attenuated proxies with cleanup of dependent assertions, subscriptions, pending calls, and references.
- [ ] [parallel] r[molten.runtime_spine.rights_amplification] Define sealer/unsealer or branded-token rights-amplification patterns for private cooperation between objects.
- [ ] [parallel] r[molten.runtime_spine.distributed_ref_lifetimes] Define session-scoped far-reference descriptors, handoff/bootstrap, and distributed lifetime or garbage-tracking rules.

## Phase 3: Persistence, upgrade, debugging, and storage

- [x] [serial] r[molten.runtime_spine.safe_object_serialization] Define safe vat/object serialization that preserves authority graphs and allows objects to provide portraits using only authority already held.
- [x] [serial] r[molten.runtime_spine.object_upgrade] Define explicit object behavior/schema versioning and upgrade recipes for restored snapshots.
- [ ] [parallel] r[molten.runtime_spine.time_travel_debugging] Define trace, snapshot, and replay hooks for time-travel and distributed object debugging.
- [ ] [parallel] r[molten.runtime_spine.authority_graph_inspection] Define an authority-aware inspection surface for object reference graphs and snapshots.
- [ ] [parallel] r[molten.runtime_spine.portable_encrypted_storage] Define content-addressed, encrypted, chunked, provider-independent storage requirements for snapshots, blobs, documents, and large payloads.

## Phase 4: Tests and integration

- [x] [serial] r[molten.runtime_spine.vat_integration_tests] Add tests for near synchronous calls, far async calls, rollback, pending action commit, reference passing, proxy revocation, and promise failure propagation.
- [x] [serial] r[molten.runtime_spine.snapshot_integration_tests] Add tests for object snapshot/restore, authority preservation, denied authority escalation, and version upgrade recipes.
- [ ] [parallel] r[molten.runtime_spine.promise_property_tests] Add Hegel property tests for bounded promise pipelines, resolution/failure ordering, and queue cleanup.
- [ ] [parallel] r[molten.runtime_spine.actormap_property_tests] Add Hegel property tests for generated actormap turn deltas preserving rollback/commit invariants.
