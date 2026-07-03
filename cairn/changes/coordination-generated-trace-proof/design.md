# Design: coordination generated trace proof

## Scope

This change strengthens coordination state-machine proof evidence with bounded generated traces. It applies to the coordination runtime, request application, receipts, state snapshots, and status assertions.

## Proof checklist

- **Proof claim**: generated coordination request traces preserve fencing monotonicity, mutual exclusion, FIFO order, semaphore bounds, duplicate replay, and no-mutation on denial.
- **Out of scope**: distributed Raft timing, network partitions, and storage adapter durability; this proof targets the deterministic in-memory control-plane state machine.
- **Trusted assumptions**: generated requests are bounded, canonical refs are valid unless a negative test intentionally malforms them, and existing control-plane commit/read-index receipts remain the authority boundary.
- **Positive evidence**: generated passing operations update state and assertions according to the primitive semantics.
- **Negative evidence**: stale tokens, capacity overflows, missing evidence, duplicate operation ids, and malformed requests deny or replay without unintended state advancement.
- **Canonical refs**: each generated step records before/after state refs, request refs, receipt refs, and assertion refs when emitted.
- **Regeneration command**: `cargo test coordination`.

## Generated trace shape

Use a pure trace-step model where possible: draw bounded operations, apply them to the coordination runtime, compare before/after state refs, and assert primitive-specific invariants after every step. The runtime shell should only build fixture values and call the pure apply path.

## Non-goals

- No claim of network-level exactly-once behavior.
- No replacement for Raft/control-plane commit evidence.
