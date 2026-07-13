## Context

Molten's content model knows whether bytes are valid and locally available, but replication requires policy over peers, placement domains, desired copies, repair urgency, transfer scheduling, and retention. Those semantics vary by workload and must not be hidden in the content adapter.

## Decisions

### 1. Replication is an optional system extension

**Choice:** A separately admitted service owns replication groups, desired replica policy, placement constraints, reconciliation cadence, handoff, repair, and status assertions. Ordinary content reads and writes do not activate replication implicitly.

**Rationale:** Local content storage is a mechanism; replica policy is service semantics.

### 2. Reconciliation planning is pure

**Choice:** Given canonical inventory, membership, placement, health, policy, resource, time, and prior-operation facts, a pure planner returns target copies, removals, repairs, deferrals, required pins, and diagnostics in deterministic order.

**Rationale:** Placement and repair decisions should be testable without a network, store, clock, or running cluster.

### 3. Transfers are receiver-driven and idempotent

**Choice:** Receivers request missing canonical refs under scoped operation ids. Exact repeats reuse prior results; conflicts deny. Senders cannot push an object into availability, pin, or import state without a matching receiver plan and local verification.

**Rationale:** Receiver control preserves local resource, policy, retention, and trust boundaries.

### 4. Placement epochs fence stale work

**Choice:** Plans and transfer operations bind membership and placement epochs plus service generation. Results from superseded epochs may contribute diagnostic availability observations but cannot satisfy current replica targets or authorize cleanup.

**Rationale:** Delayed repair and handoff work must not undo a newer placement decision.

### 5. Repair never bypasses retention or confidentiality

**Choice:** Active copies acquire canonical retention pins before transfer or source cleanup. Protected commitments, encryption transforms, and reveal requirements remain content-policy inputs; replication does not decrypt or expose data merely to verify placement.

**Rationale:** Availability work must not weaken deletion or confidentiality rules.

### 6. Evidence is aggregate and bounded

**Choice:** Emit evidence for manifest admission, reconciliation plans, material transfer/repair outcomes, under-replication, placement epoch changes, and cleanup decisions. Per-frame and per-read receipts are not required.

**Rationale:** Replication must remain observable without becoming receipt-bound on its hot path.

## Functional core / imperative shell split

- Pure core: inventory diff, deterministic target selection, placement and repair plans, idempotency, epoch fencing, retention requirements, convergence checks, and receipt payloads.
- Shell: observe inventories, schedule timers, request transfers, stream through content adapters, persist status, manage pins, supervise retries, and publish bounded assertions/readback.

## Dependencies

- System-extension runtime.
- Content-store adapters.
- Fabric transport, durable-state, time, membership/placement, identity, resource, observability, and simulation profiles.

## Risks / Trade-offs

- Replica counts can be misread as durability guarantees. Bind evidence to observed peers, time, failure model, and verification scope.
- Repair storms can amplify failures. Enforce concurrency, byte, peer, and retry budgets with deterministic admission.
- Placement policy may evolve. Version policies and fence all in-flight operations by epoch.
