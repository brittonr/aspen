## Context

Membership `ResourceAmount` and `NodeDescriptor` have stable CPU/memory/storage semantics. Optional compute service records must compose with membership's host identity and label authority, without converting advisory membership placement into device allocation authority.

## Decisions

### Decision: Separate versioned offers and reservations

**Choice:** Offer snapshots bind provider and source profiles, host/device incarnation, topology generation, revision, explicit observation and expiry inputs, health, qualification, physical pools and allocation views. Reservations bind consumer, operation, attempt, generation, grant, claims, status and enforcement independently of worker records.

**Rationale:** Membership v1 encodings remain unchanged. Compatibility unknowns, absent observations and unsupported groupings cannot masquerade as qualified offers.

### Decision: One selected authority per backing pool

**Choice:** Every view resolves to explicit physical pool identities and their selected authority. A selected serialized owner persists its immutable identity in the durable store, applies a pure reserve transition to its current state, checks all pools, and persists intent before backend activation. The shell replaces caller-supplied grant/revocation, minimum label authority and current time with admitted owner and clock ports before deciding. A duplicate operation returns the existing allocation.

**Rationale:** Distinct aliases, partitions, chips, trays and meshes cannot independently spend the same pool. A deployment owns resident charges once; remote consumers refer to it without copying its charges. An offer from another selected pool owner cannot authorize this store to spend it.

### Decision: Uncertainty retains charges

**Choice:** Drain/cancel/timeout/lost acknowledgements and disappearance quarantine rather than free. Owner generation plus consumer and positive release evidence queried from the selected backend owner are required to discharge claims; caller-authored evidence cannot reconcile. A definite no-effect activation refusal retires its intent and frees the charge only after that fact is persisted. Readback distinguishes logical fencing from backend enforcement.

**Rationale:** A stop request or expired lease is not physical release proof. Reconciliation cannot convert possible effects into false success or repeat a backend start/stop. Released allocations move to bounded FIFO tombstones: replay is rejected while retained; evicted operation IDs become unknown and may be admitted anew.

### Decision: Capability-rooted durable owner and effect ordering

**Choice:** A single node-local Redb owner opens under an admitted `DurableStoreRoot` and pins its selected authority identity. Each compare-and-swap commits canonical packed Preserves state (offers, active allocations, monotonic charge cursor and bounded retired records) in one transaction. An offer captures its observer's charge cursor rather than receiving a new basis on delivery; newer active charges remain debited even if observation already reports older charges. The shell persists reservation intent, calls the backend, and records activation only after matching backend evidence. Reopen never replays a pending backend operation; reconciliation queries backend-owned read-only activation/release observations and compares exact evidence identities.

**Rationale:** A delayed observation, interrupted start, failed commit or lost stop acknowledgement must retain its charge. The store is node-local exclusion, not globally distributed coordination; independent store copies cannot spend the same physical pool.

### Decision: Separate public canonical records and profile contracts

**Choice:** New versioned Preserves offer, allocation and readback records use BLAKE3 over canonical packed bytes; strict decoders reject unknown labels, arity and malformed refs. Versioned Nickel profiles declare provenance and supported granularity without extending membership v1.

**Rationale:** Consumers can bind exact canonical evidence without changing frozen membership compatibility fixtures.

### Decision: Explicit non-claims

**Choice:** The supplied-observation adapter and node-local store do not provide live hardware probing or device enforcement, TT grouping qualification, distributed allocation authority, inference throughput, model readiness or per-request capacity.

**Rationale:** Synthetic fixtures and restart checks establish accounting and uncertainty boundaries, not device qualification.

## Risks / Trade-offs

- Serialization across physical pools requires a selected admitted shell/store authority. An arbitrary caller can implement a port, so deployment wiring must supply grant and clock owners rather than request-derived stand-ins; independent callers applying the same immutable snapshot cannot establish concurrency safety.
- Physical identity alias accuracy and observation charge cursor require provider qualification; unsupported grouping is denied rather than inferred. The FIFO retirement bound admits operation-ID reuse after eviction and requires an external owner to scope replay outside that horizon.
