## Why

Distributed services depend on timers, deadlines, retries, leases, periodic work, logical clocks, and nondeterministic choices. If extension code reads ambient clocks, sleeps directly, or draws ambient randomness, live execution cannot be reproduced under deterministic simulation and stale timers can escape service-generation fencing.

Aspen needs explicit time, timer, scheduler, and entropy ports whose production and simulation implementations share one observable contract.

## What Changes

- Define canonical wall-clock observation, monotonic duration, logical event time, timer, deadline, and clock-uncertainty values.
- Add generation-scoped one-shot and periodic timer operations with explicit ordering, cancellation, coalescing, lateness, and overload behavior.
- Add a deterministic runnable scheduler contract for extension tasks, callback wakeups, and controlled interleavings.
- Add deterministic entropy and choice streams bound to run, service generation, and replay position.
- Define lease and deadline rules that consume explicit clock assumptions instead of treating local time as distributed authority.
- Provide live and deterministic-simulation adapters with shared conformance and bounded evidence.

## Impact

- **Files**: canonical time and scheduling models, runtime dispatcher, live clock/timer shell, deterministic scheduler and entropy adapters, service-generation cleanup, operator readback, fixtures, and a new `fabric-time-scheduling` accepted spec.
- **Testing**: timer ordering, cancellation, periodic coalescing, deadlines, clock jumps, uncertainty, stale generations, deterministic entropy, replay, scheduler fairness bounds, overload, and lease-expiry tests.
- **Safety**: a local clock or timer event does not prove global time, remote expiry, distributed lease exclusivity, fairness, liveness, or safe retry.
