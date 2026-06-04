## Phase 1: Dataspace interaction semantics

- [ ] [serial] r[molten.runtime_spine.synit_reference_boundary] Document Synit and the Syndicated Actor Model as non-normative design references, not compatibility targets.
- [ ] [serial] r[molten.runtime_spine.turn_semantics] Define actor turn semantics with pending action accumulation and commit/rollback behavior.
- [ ] [serial] r[molten.runtime_spine.assertion_lifetimes] Define assertion ownership, handles, automatic retraction on actor/session termination, and duplicate assertion deduplication.
- [ ] [serial] r[molten.runtime_spine.observe_patterns] Define `Observe`-style subscription assertions over Preserves patterns.
- [ ] [parallel] r[molten.runtime_spine.preserves_patterns] Define a deterministic, bounded Preserves pattern language for dataspace routing and policy-visible matching.

## Phase 2: Authority and references

- [ ] [serial] r[molten.runtime_spine.capability_attenuation] Define capability attenuation over messages, assertions, subscriptions, and reference introduction using Molten policy gates.
- [ ] [serial] r[molten.runtime_spine.gatekeeper_resolver] Define a gatekeeper resolver for converting long-lived credentials into live scoped references with attenuation, expiry, and evidence.
- [ ] [parallel] r[molten.runtime_spine.reference_lifetimes] Define live reference lifetime, revocation, and cleanup semantics for local actors, dataspaces, protocol sessions, consensus resources, blobs, and host resources.

## Phase 3: Services and supervision

- [ ] [serial] r[molten.runtime_spine.service_dependency_assertions] Define service lifecycle and dependency assertions for demand-driven startup, readiness, restart, failure, completion, and exposed service objects.
- [ ] [parallel] r[molten.runtime_spine.supervision_tree] Define logical supervision relationships separately from OS process parentage and adapter-specific process supervision.
- [ ] [parallel] r[molten.runtime_spine.demand_driven_startup] Implement local demand-driven startup/shutdown behavior over the dataspace model without hardcoded service graphs.

## Phase 4: Tracing and tests

- [ ] [serial] r[molten.runtime_spine.interaction_tracing] Define canonical Preserves trace records for turns, assertions, retractions, messages, policy decisions, choreography transitions, consensus events, and receipts.
- [ ] [parallel] r[molten.runtime_spine.trace_rendering] Add an inspection/export surface that can filter trace records and support later sequence-diagram rendering.
- [ ] [serial] r[molten.runtime_spine.sam_integration_tests] Add tests for turn rollback, assertion auto-retraction, Observe delivery/retraction, attenuation deny/rewrite/admit, gatekeeper resolution, service dependency startup, and trace emission.
- [ ] [parallel] r[molten.runtime_spine.sam_property_tests] Add Hegel property tests for generated assertion/update/retraction and subscription sequences within supported bounds.
