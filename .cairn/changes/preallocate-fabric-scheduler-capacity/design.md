## Context

The admitted fabric-time profile already contains `max_runnables`, `max_scheduler_queue_depth`, and `max_scheduler_concurrency`.

The pure scheduler checks these values during transitions. Its runtime state currently creates `runnables` with `Vec::new()`.

Logical bounds and physical capacity are separate facts. This change connects them for the selected scheduler hot path.

## Decisions

### 1. The admitted profile is the only capacity source

**Choice:** Derive a pure `SchedulerCapacityPlan` from the admitted profile and current system-extension generation.

The plan checks count conversion, queue relationships, concurrency relationships, and total allocation arithmetic.

**Rationale:** A second capacity configuration can drift from runtime policy.

### 2. Activation allocates capacity before scheduling

**Choice:** The shell reserves the planned runnable and queue capacity before it exposes the scheduler as active.

Allocation failure produces a typed activation denial. It does not start a smaller scheduler or use a hidden fallback.

**Rationale:** An admitted profile must not become active when its selected scheduler capacity is unavailable.

### 3. Capacity is generation-fenced

**Choice:** Runtime capacity carries the active extension generation and profile reference. Restart, upgrade, rollback, or replacement creates new capacity ownership.

Stale capacity cannot accept wake, choose, yield, block, complete, or cancellation transitions.

**Rationale:** Reused memory must not weaken existing generation fences.

### 4. Steady-state transitions cannot widen capacity

**Choice:** Enqueue, wake, choose, and replay transitions check logical and physical capacity before mutation.

If capacity is exhausted, the existing profile overload policy selects reject, backpressure, or recorded drop. No vector growth fallback occurs.

**Rationale:** Physical exhaustion must remain consistent with declared scheduler policy.

### 5. Ordering and replay stay unchanged

**Choice:** Preserve existing FIFO or priority/FIFO ordering, fairness promotion, recorded-choice replay, and deterministic evidence order.

Capacity state is not a new ordering input except when declared exhaustion invokes existing overload policy.

### 6. Observations remain bounded runtime facts

**Choice:** Record plan identity, activation result, current usage, high-water usage, exhaustion counts, releases, generation, and profile reference.

Do not promote profiling traces or allocation counters into authority, fairness, liveness, or release claims.

## Functional Core and Imperative Shell

The pure core validates the plan, checks generation and profile identity, checks capacity transitions, and classifies exhaustion.

The shell allocates capacity, owns memory, drives transitions, writes observations, and releases capacity during cleanup.

## Risks and Trade-offs

- Large admitted limits can reserve significant memory before work starts.
- Retained capacity can reduce memory available to other runtime services.
- Profile values can require tighter operator guidance after physical enforcement.
- Instrumented allocation builds change the measured runtime.

## Validation

Positive tests cover exact-limit activation, enqueue and completion cycles, replay parity, fairness parity, cleanup, and restart.

Negative tests cover allocation failure, one-past-limit plans, stale generations, wrong profiles, exhaustion, hidden fallback growth, and observation overclaims.
