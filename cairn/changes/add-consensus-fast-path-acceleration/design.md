## Context

Molten's accepted architecture supports pluggable algorithm and implementation profiles, canonical commands, normalized commit/read evidence, extension-owned pure state machines, client-session idempotency, engine epochs, fencing, and fail-closed production admission. The active live-consistency change is blocked because the admitted transport port cannot yet connect long-lived distinct replica processes. The separate `model-consensus-fast-path-hazards` package captures fast-path composition invariants without making live claims.

Jetpack is a useful implementation and evaluation reference at pinned MIT-licensed commit `c03e318ec355b11edd42aac56c68d0765f88d1d2`. Its C++/WAF runtime is not a suitable Molten dependency, and its TLA+ and AWS results are not proof or benchmark evidence for Molten. This design therefore specifies an independently implemented optional acceleration profile after the base service exists.

## Decisions

### 1. Acceleration composes with a base engine; it is not a new authority layer

**Choice:** Register a composite implementation profile that binds an exact base algorithm, base implementation profile, acceleration implementation, compatibility cohort, conflict contract, quorum/recovery policy, environment scope, and evidence refs. The base engine remains the canonical ordering and recovery authority. Compatibility must explicitly establish that conflicting commands from one proposer preserve proposal order in log/execution order and that proposer receive order preserves proposal order. A buffering layer that can reorder receipt and proposal requires the fast reply to wait for equivalent proposal-order evidence; an engine that can reorder conflicting proposals at execution is incompatible with the transparent profile.

**Rationale:** Treating the fast path as an unrelated engine would obscure its dependency on base receive/propose/execute ordering, view transitions, durable recovery, and eventual convergence.

### 2. The feature is opt-in and default-off

**Choice:** Group manifests must explicitly select an admitted acceleration profile. Unknown, incompatible, evidence-incomplete, or unavailable acceleration denies or falls back according to typed policy without silently changing the base engine. Disabling acceleration sends no fast-path protocol traffic and preserves the base path's declared behavior.

**Rationale:** Latency optimization must not become a hidden availability or correctness dependency.

### 3. Extensions own conflict semantics through a narrow pure contract

**Choice:** The application state-machine manifest may bind a versioned pure conflict classifier over canonical commands. The classifier has no transport, log, leader, clock, storage, policy mutation, or runtime handles. Unknown or unsupported dependencies conservatively fall back. Production admission requires semantic fixtures and model evidence for the declared command domain.

**Rationale:** Generic node core cannot infer whether order changes application state or replies. A false negative is safety-critical, while conservative fallback is safe.

### 4. Engine internals remain opaque to extensions

**Choice:** Extensions continue to receive normalized commit, denial, retryable, cancelled, or uncertain outcomes. Fast/original path races, proposer identity, views, quorums, and recovery internals remain engine-private evidence. Both paths use one canonical operation identity and only the committed state-machine boundary can authorize extension mutation.

**Rationale:** Portability requires application semantics not to depend on which path won or on internal consensus topology.

### 5. Adaptive routing is typed policy over bounded telemetry

**Choice:** A pure routing core chooses original-only or fast-attempt from a typed Nickel policy, declared topology, recent bounded latency/resource/conflict observations, and current fast-path health. The shell gathers measurements and executes the selected path. Thresholds, windows, probe rates, and backoff bounds are named configuration fields; the Jetpack paper's heuristic is reference data, not a default.

**Rationale:** The fast path helps only when RTT dominates and capacity is available. Hidden constants or ambient CPU observations would make behavior irreproducible and difficult to review.

### 6. Recovery follows the checked model contract

**Choice:** Live acceleration must preserve same-view acknowledgements, all-active-proposer promises, an independent acceleration view, recoverable prior-view commands, recovery-set agreement, original-path recovery/no-op markers, recovery-before-new-view admission, accepted-set carry-forward, and at-most-once convergence.

**Rationale:** These are composition obligations beyond base Raft safety and require their own implementation, simulation, and live failure evidence.

### 7. Initial capability scope is deliberately narrow

**Choice:** The first candidate profile supports the exact static-membership crash-fault Raft cohort and command classes proven by its conflict contract. It denies dynamic membership, leadership transfer, Byzantine faults, interactive transactions, cross-group atomicity, unbound range/predicate semantics, and unsupported read modes.

**Rationale:** Each additional capability changes proposer, view, conflict, recovery, or quorum assumptions and requires separate evidence.

### 8. Production admission requires benefit and non-regression

**Choice:** Admission binds an exact implementation, topology, placement, membership, transport, durability, workload, fault model, resource envelope, and environment. It requires: an admitted live base engine; passing model and deterministic simulation; distinct-process commit/recovery evidence; original-only equivalence with acceleration disabled; safe fallback under conflict and failure; bounded recovery impact; resource limits; and measured latency benefit without unacceptable throughput, tail-latency, or availability regression. A three-replica profile must surface that any replica loss disables its fast path.

**Rationale:** External AWS averages do not establish value for Iroh transport, Molten policy/evidence costs, control-plane workloads, or operator SLOs.

### 9. Evidence remains off packet hot paths

**Choice:** Emit canonical evidence for profile/group admission, selected fast commits or ranges, fallback classes, recovery sets and markers, view/epoch transitions, aggregate attempt/success/resource metrics, failures, and benchmark decisions. Do not require standalone authority receipts for every fast acknowledgement, replication packet, heartbeat, or timer event.

**Rationale:** Per-message receipts would distort the optimization while adding no corresponding application authority.

## Functional core / imperative shell split

- Pure core: descriptor compatibility, conflict classification, quorum/view checks, routing policy, fallback decision, recovery planning, duplicate suppression, normalized outcome construction, evidence payloads, benchmark comparison, and production-admission decisions.
- Shell: bind the admitted base engine and fabric ports, broadcast fast requests, submit original-path requests, gather replies and telemetry, execute recovery messages, persist bounded state/evidence, supervise the service, and expose operator workflows.

## Dependencies

- `model-consensus-fast-path-hazards` completed, validated, synced, and archived.
- `fabric-consistency-service-runtime` completed with an exact production-admitted live Raft implementation.
- `fabric-whole-system-simulation` capable of same-core consistency fault simulation and replay.
- Receipt-first cluster harness and admitted live transport, durable-state, time, membership, placement, fencing, supervision, and resource ports.

## Risks / Trade-offs

- A fast path increases CPU, network fanout, state, and recovery complexity while reducing latency only for suitable topology and contention. Production admission must be evidence-driven and reversible.
- Conservative conflict classification can make the optimization rarely useful; aggressive classification can break safety. Keep the contract extension-owned, fail-closed, and profile-scoped.
- Three-replica groups lose fast-path availability after one replica failure even though Raft retains majority availability. Status and routing must distinguish the two.
- Recovery priority pauses new-view work and may worsen failover latency. Bind acceptance to explicit operator recovery objectives rather than normal-case averages alone.
- A composite profile can be mistaken for generic Jetpack compatibility. Scope every claim to the exact base implementation, command domain, membership, environment, and evidence cohort.

## Blocker

Do not implement or sync this change while its prerequisites are incomplete. The current transport shell cannot connect long-lived distinct replica processes, the live Raft service is not production-admitted, and the fast-path hazard model is not implemented. External Jetpack code, TLA+ checks, or benchmark results cannot discharge those blockers.
