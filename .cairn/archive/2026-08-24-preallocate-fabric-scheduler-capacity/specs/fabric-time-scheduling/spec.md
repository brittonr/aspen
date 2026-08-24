# Fabric Time Scheduling Specification Delta

## ADDED Requirements

### Requirement: Scheduler capacity derives from the admitted profile

r[molten.fabric_time.scheduler_capacity.plan]

Molten MUST derive one checked scheduler-capacity plan from the admitted fabric-time profile and active system-extension generation.

#### Scenario: A valid profile produces a plan

- GIVEN an admitted profile has coherent runnable, queue, and concurrency limits
- WHEN scheduler-capacity planning runs
- THEN the plan MUST bind those limits, the profile reference, and the active generation
- AND every count and allocation calculation MUST use checked conversion and arithmetic

#### Scenario: A profile cannot produce capacity

- GIVEN a limit is unrepresentable, contradictory, or above its compiled hard cap
- WHEN scheduler-capacity planning runs
- THEN scheduler activation MUST be denied before allocation or runnable mutation

### Requirement: Capacity is allocated before activation

r[molten.fabric_time.scheduler_capacity.activation]

The live scheduler shell MUST allocate planned runnable and queue capacity before exposing the scheduler as active.

#### Scenario: Startup capacity is available

- GIVEN the checked plan fits available memory
- WHEN scheduler activation runs
- THEN the runtime MUST own the complete selected capacity before accepting work

#### Scenario: Startup allocation fails

- GIVEN any planned scheduler capacity cannot be allocated
- WHEN scheduler activation runs
- THEN activation MUST return a typed denial
- AND it MUST NOT start a smaller scheduler or select a hidden fallback

### Requirement: Steady-state scheduler transitions do not widen capacity

r[molten.fabric_time.scheduler_capacity.steady_state]

An active scheduler MUST use initialized capacity for wake, enqueue, choose, yield, block, complete, cancel, and replay transitions.

#### Scenario: Work remains within capacity

- GIVEN an active matching profile and generation with free capacity
- WHEN valid scheduler transitions run
- THEN transitions MUST use existing capacity
- AND ordering and replay outcomes MUST remain unchanged

#### Scenario: Physical capacity is exhausted

- GIVEN no free admitted slot remains
- WHEN another runnable requires capacity
- THEN the existing overload policy MUST decide reject, backpressure, or recorded drop
- AND the scheduler MUST NOT grow its backing capacity

### Requirement: Capacity remains profile-bound and generation-fenced

r[molten.fabric_time.scheduler_capacity.boundary]

Scheduler capacity MUST reject use by another profile or stale system-extension generation.

#### Scenario: A generation is replaced

- GIVEN restart, upgrade, rollback, or replacement changes the active generation
- WHEN old capacity receives a scheduler transition
- THEN the transition MUST fail as stale
- AND old state MUST NOT enter the new generation

#### Scenario: Existing scheduling policy runs

- GIVEN initialized capacity and an admitted FIFO, priority, fairness, or recorded-choice policy
- WHEN the scheduler selects work
- THEN current ordering, fairness, replay, and cleanup rules MUST remain authoritative

### Requirement: Capacity observations stay scoped

r[molten.fabric_time.scheduler_capacity.observation]

Capacity observations MUST report only plan, activation, usage, high-water, exhaustion, release, profile, and generation facts.

#### Scenario: A capacity observation is reviewed

- GIVEN a scheduler emitted capacity observations
- WHEN evidence validation runs
- THEN the observations MUST match the selected plan and transitions
- AND they MUST NOT claim global latency, fairness, liveness, host memory stability, or whole-runtime zero allocation

### Requirement: Verification covers positive and negative capacity paths

r[molten.fabric_time.scheduler_capacity.verification]

Implementation MUST pair successful scheduler-capacity tests with allocation, identity, exhaustion, cleanup, replay, and overclaim failures.

#### Scenario: Focused scheduler verification runs

- GIVEN valid and invalid capacity fixtures
- WHEN focused fabric-time verification runs
- THEN valid ordering and replay results MUST remain unchanged
- AND every invalid capacity path MUST fail with its declared typed result
