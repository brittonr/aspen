# Design: Exercise simulation faults causally

## Context

The reference composition routes every effect through `DeterministicSimulationPortRouter`, which folds the active fault into accounting and output identity but always returns success. The pure core already decides which transitions and effects are legal; only the simulated shell lacks stateful completion behavior.

## Approach

### Stateful virtual transport

The transport adapter owns a bounded pending set. `Delay` moves an event's eligibility to a later virtual tick. `Drop` removes an eligible delivery. A partition blocks selected destinations until a heal event. Deliveries become scheduler-eligible events through the same boundaries the live shell uses, so the pure core cannot distinguish simulated completion events from real ones.

### Stateful virtual storage

The storage adapter separates three images: submitted operations not yet applied, completed operations, and the durable image visible after recovery. A completion fault holds an operation in the submitted set. A crash drops in-memory service state and rebuilds from the durable image only. The adapter models the declared storage contract, including transactional Redb commit and `OutcomeUnknown` semantics; it does not emulate filesystem internals.

### Causal acceptance test

The acceptance test for the whole change: with a delayed storage completion, the service must be able to acknowledge a request before the corresponding write becomes recoverable, so a crash after the acknowledgment loses the acknowledged state. If the fault only changes a receipt hash and never the acknowledgment outcome, the change is not done.

### Exploration, replay, minimization

- Exploration: the runner selects among causally eligible events using a recorded seeded choice instead of passing `None`.
- Replay: the comparator compares virtual ticks, generations, semantic outputs, and choice records, and reports the first mismatching field.
- Minimization: the shrink predicate reruns the candidate under the recorded environment and retains it only when the same failure fingerprint occurs; the label-only predicate is removed.

## Alternatives considered

- Keep the router stateless and only enrich event records. Rejected: this preserves the current defect where faults cannot change outcomes.
- Model a full network or filesystem. Rejected: bounded contract-level behavior is sufficient and keeps the simulation deterministic and auditable.

## Non-claims

- Simulation results do not prove live-network or live-storage equivalence.
- Virtual-time behavior does not benchmark real Iroh, Redb, or OS performance.
- A passing minimized case does not prove the absence of other failures.

## Risks

- Stateful adapters change reference-run outcomes and evidence refs; fixtures regenerate in the same change.
- Seeded exploration can surface pre-existing invariant failures; each new failure is triaged before the change closes.
