## Context

World operations span immutable publication, mutable compare-and-swap, external witness or effect requests, replication, and retention. No universal transaction can cover all of them.

Each subsystem already owns its decision and recovery meaning. This change adds a common conformance harness without moving that meaning into test code.

## Decisions

### Decision: Maintain a closed mutation-boundary inventory

**Choice:** Define a versioned inventory for every supported world mutation. Each row names the owning component, operation identity domain, expected pre-state, mutation effects, linearization point, durable completion record, uncertain window, reconciliation entry point, and required negative cases.

An unregistered mutation cannot claim world crash-conformance coverage.

**Rationale:** Fault tests are incomplete when new writes can appear without explicit scenarios.

### Decision: Inject faults at semantic boundaries

**Choice:** Test profiles use named phases such as before submit, after possible submit, after durable write, before response, lost response, process restart, and recovery read-back.

The profile does not use source-line hooks as semantic authority. Adapter-specific hooks map to the named phases and record their identity.

**Rationale:** Source locations change. Semantic operation phases remain reviewable across refactors.

### Decision: Keep expected decisions in owning cores

**Choice:** The harness supplies observations to world cores and Transactional Reconciliation Core. It compares returned decisions and durable read-back with the inventory's expected classes.

The test shell does not invent success, failure, or compensation semantics.

**Rationale:** A test-only state machine would duplicate product policy and could mask defects.

### Decision: Model concurrency with explicit schedules

**Choice:** Deterministic schedules cover competing head claims, branch promotions, witness finalization, effect reservations, capsule imports, replication updates, retention pins, and garbage-collection plans.

Every operation names expected generations, operation IDs, and declared interleaving points. Wall-clock timing does not select outcomes.

**Rationale:** Repeatable schedules expose races without treating thread timing as semantic evidence.

### Decision: Require conservative restart classification

**Choice:** Recovery observes durable records and returns already-complete, safe-to-retry, superseded, conflict, uncertain, denied, corrupt, or manual-review.

Missing or contradictory records cannot become success. Cleanup cannot execute from incomplete reachability or authority facts.

**Rationale:** Restart is an admission boundary, not a license to reconstruct optimistic state.

### Decision: Separate local rollback from witnessed rollback

**Choice:** Local-store image rollback tests must report undetected or unproven under the local profile. Strong rollback tests require independent witness state that is not rolled back with the store.

**Rationale:** A local harness must not manufacture an external currentness guarantee.

### Decision: Bind fault evidence without overclaiming

**Choice:** A conformance receipt records source revision, inventory identity, fault profile, adapter identities, schedule, limits, cases, durable observations, decisions, and unsupported rows.

Passing proves only bounded agreement for the exercised cohort and fault points.

**Rationale:** Crash injection is evidence, not proof of every physical failure mode.

## Rollout

1. Inventory all world mutation boundaries before adding fault hooks.
2. Add pure fixture tests for each operation and reconciliation class.
3. Add deterministic in-memory interruption and concurrency tests.
4. Add process-restart tests over local durable adapters.
5. Add witness and effect uncertainty cases through fake and one reviewed live adapter.
6. Add the bounded matrix to the world operator dogfood rail.

## Risks / Trade-offs

- The matrix can grow quickly. Closed profiles and named cohorts keep it bounded.
- Fault hooks can alter timing. Semantic phase receipts make the intervention visible.
- Filesystem simulation cannot prove physical power-loss behavior. Live storage lanes remain separate evidence.
- Concurrent schedules can become redundant. Keep a minimal positive and adversarial basis per mutation row.
