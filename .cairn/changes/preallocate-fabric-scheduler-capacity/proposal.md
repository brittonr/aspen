## Why

Molten already admits fabric-time profiles with hard limits for runnables, queue depth, scheduler concurrency, and fairness.

The scheduler state still starts with an empty runnable vector. Logical admission prevents over-bound state, but it does not make admitted capacity available before activation.

The scheduler needs a profile-bound startup-capacity step. Steady-state enqueue and selection must not depend on vector growth.

## What Changes

- Add a pure scheduler-capacity plan derived from one admitted fabric-time profile.
- Allocate runnable and selected queue capacity before the scheduler becomes active.
- Bind capacity to the exact profile and system-extension generation.
- Reject activation when capacity cannot be allocated.
- Prevent steady-state scheduler transitions from widening capacity.
- Add scoped allocation, high-water, exhaustion, generation, and replay observations.

## Impact

- **Core**: capacity-plan checks, generation binding, high-water accounting, and typed exhaustion outcomes.
- **Shell**: fallible startup allocation, runtime ownership, teardown, and observation persistence.
- **Configuration**: existing admitted profile fields remain authoritative. No second capacity configuration is added.
- **Evidence**: observations report one scheduler instance and do not prove global latency, fairness, liveness, or host memory stability.

## Dependencies

The active development profiling change can measure candidate paths, but it is not runtime authority. This change remains valid without profiler availability.

## Non-Goals

- Do not preallocate every Molten collection.
- Do not make all vats, extensions, or transports use one global scheduler thread.
- Do not change scheduler ordering, replay choices, fairness rules, or overload policy.
- Do not replace Preserves with native Rust layout.
- Do not add direct I/O, `io_uring`, or a general allocator.
