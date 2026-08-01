## Context

The accepted testing-harness spec provides canonical suites, replay, effect logs, faults, adapter conformance, and multi-peer concepts. The fabric changes make stronger same-code simulation possible because extension effects now cross explicit ports. This change defines the composition and proof boundary for a complete distributed service run.

## Decisions

### 1. Simulation replaces shells, not extension cores

**Choice:** The simulation composition loads the same system-extension artifact identity, manifest, callback dispatcher, pure protocol/state-transition core, application state machine, schemas, and port command/event types used by live execution. It substitutes only admitted shell implementations and the top-level scheduler.

**Rationale:** A separately implemented mock validates the mock, not the service intended for production.

### 2. The simulated world is a canonical artifact

**Choice:** A world manifest binds node descriptors, extension generations, port profiles, membership views, placement and consistency profiles, initial durable state, resources, workload, scheduler and entropy inputs, fault plan, invariants, bounds, runtime identity, and non-claims.

**Rationale:** Every source of behavior must be inspectable, hashable, and replayable.

### 3. Faults occur at named port and lifecycle boundaries

**Choice:** The fault engine can delay, drop, duplicate, reorder, partition, reset, corrupt, exhaust, pause, crash, restart, skew, jump, expire, revoke, and replace only through declared adapter or lifecycle transitions. It cannot mutate extension state invisibly.

**Rationale:** Named boundaries preserve causality and make counterexamples portable.

### 4. One scheduler controls all nondeterministic choices

**Choice:** The deterministic scheduler controls runnable selection, message delivery, timer firing at eligible virtual time, disk completion, process lifecycle completion, and fault activation. Every choice has a canonical position and bounded alternative set.

**Rationale:** Replay and systematic exploration require a single explicit choice stream.

### 5. Invariants observe canonical histories and states

**Choice:** Extensions register pure invariant and history-check functions over redacted canonical observations. The harness also checks universal invariants: no ambient effects, no stale-generation mutation, no bound bypass, no impossible port transition, no invalid content ref, and cleanup completeness.

**Rationale:** Service semantics remain extension-owned while fabric safety invariants remain reusable.

### 6. Shrinking preserves causal validity

**Choice:** Shrinkers may reduce workloads, nodes when the property remains meaningful, fault actions, delays, scheduler choices, and resource envelopes. Every candidate is replayed from the canonical initial world; invalid candidates are rejected, not repaired invisibly.

**Rationale:** Small causal counterexamples are more useful than raw random traces.

### 7. Three reference slices are the architecture exit test

**Choice:** Implement bounded witness extensions for a transactional ordered key-value service, a replicated append log, and a distributed scheduler. Each uses only system-extension callbacks and fabric ports. The slices prove mechanism sufficiency, not external product compatibility or production completeness.

**Rationale:** Their different state, ordering, placement, and recovery needs expose workload-specific leakage into core.

### 8. Evidence profiles form a claim ladder

**Choice:** Label evidence as pure model, deterministic whole-system simulation, multi-process live, host-chaos, or VM/hardware profile. Stronger gates may consume weaker evidence but must also require profile-specific evidence; simulation cannot be relabeled as live.

**Rationale:** Honest profile boundaries prevent deterministic success from becoming an operational claim.

## Functional core / imperative shell split

- Pure core: world validation, scheduler and fault transitions, port state machines, workload generation and shrinking, invariant evaluation, replay comparison, divergence selection, state/history hashing, and claim-ladder decisions.
- Shell: load artifacts, instantiate simulated or live adapters, run the event loop, persist run directories and repro bundles, invoke optional multi-process harnesses, and render bounded reports.

## Dependencies

- System-extension runtime and all fabric mechanism-port changes.
- Receipt-first cluster harness for shared run-directory, multi-process, lifecycle, and offline-verification conventions.

## Risks / Trade-offs

- A simulator can faithfully implement the wrong model. Require shared adapter conformance and differential live tests for overlapping semantics.
- State-space exploration can explode. Enforce explicit bounds, partial-order or workload reductions only when reviewed, and retain unexplored-coverage metrics.
- Reference slices can grow into products. Keep manifests and acceptance scenarios minimal and move product-specific expansion to separate changes.
