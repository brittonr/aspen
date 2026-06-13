## Phase 1: Dataspace interaction semantics

- [x] [serial] r[molten.runtime_spine.synit_reference_boundary] Document Synit and the Syndicated Actor Model as non-normative design references, not compatibility targets.
- [x] [serial] r[molten.runtime_spine.turn_semantics] Define actor turn semantics with pending action accumulation and commit/rollback behavior.
- [x] [serial] r[molten.runtime_spine.assertion_lifetimes] Define assertion ownership, cleanup, automatic retraction on actor/session termination or authority loss, and duplicate assertion visibility semantics.
- [x] [serial] r[molten.runtime_spine.observe_patterns] Define `Observe`-style subscription assertions over the implemented Preserves pattern subset.
- [x] [parallel] r[molten.runtime_spine.preserves_patterns] Define deterministic, bounded exact-value and wildcard Preserves pattern matching for dataspace routing and policy-visible matching; richer compound patterns remain future extensions.

## Phase 2: Authority and references

- [x] [serial] r[molten.runtime_spine.capability_attenuation] Define capability attenuation over messages, assertions, subscriptions, and reference introduction using Molten policy/authority gates, with rewrite transforms deferred to explicit future rule evidence.
- [x] [serial] r[molten.runtime_spine.gatekeeper_resolver] Define a gatekeeper resolver for converting long-lived credentials or authority contexts into live scoped references with attenuation, expiry, and evidence.
- [x] [parallel] r[molten.runtime_spine.reference_lifetimes] Define live reference lifetime, revocation, and cleanup semantics for local actors, dataspaces, protocol sessions, consensus resources, blobs, and host resources where implemented.

## Phase 3: Services and supervision

- [x] [serial] r[molten.runtime_spine.service_dependency_assertions] Define service lifecycle and dependency assertions for demand-driven startup, readiness, restart, failure, completion, and exposed service objects.
- [x] [parallel] r[molten.runtime_spine.supervision_tree] Define logical supervision relationships separately from OS process parentage and adapter-specific process supervision.
- [x] [parallel] r[molten.runtime_spine.demand_driven_startup] Implement local demand-driven startup/shutdown behavior over the dataspace/service evidence model without hardcoded service graphs.

## Phase 4: Tracing and tests

- [x] [serial] r[molten.runtime_spine.interaction_tracing] Define canonical Preserves trace/report records for turns, assertions, retractions, messages, policy decisions, choreography transitions, consensus events, and receipts where implemented.
- [x] [parallel] r[molten.runtime_spine.trace_rendering] Add inspection/export/summary surfaces that can render canonical trace/report records without replacing evidence.
- [x] [serial] r[molten.runtime_spine.sam_integration_tests] Add tests for turn rollback, assertion auto-retraction/cleanup, Observe delivery/retraction, attenuation denial, gatekeeper resolution, service dependency startup, supervision cleanup, and trace emission.
- [x] [parallel] r[molten.runtime_spine.sam_property_tests] Add bounded Hegel/property tests for generated assertion/update/retraction, subscription, service dependency, and owner-lifetime sequences within supported bounds.
