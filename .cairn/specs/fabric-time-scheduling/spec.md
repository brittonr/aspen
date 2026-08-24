# Fabric Time Scheduling Specification

## Purpose

Defines the `fabric-time-scheduling` capability.

## Requirements

### Requirement: Time domains are canonical and distinct
r[molten.fabric_time.time_domains] Aspen MUST represent wall-clock observations, monotonic instants and durations, logical event positions, and deterministic virtual time as distinct canonical types. Arithmetic, comparison, conversion, serialization, uncertainty, range, and overflow behavior MUST be explicit. Extension code MUST NOT read ambient clocks or treat values from different domains as interchangeable.

#### Scenario: Monotonic deadline is evaluated safely
- GIVEN a callback receives a monotonic start and checked duration from one admitted time profile
- WHEN it computes a local deadline
- THEN arithmetic either returns a valid monotonic deadline or a bounded error without wrapping.

#### Scenario: Mixed-domain comparison denies
- GIVEN extension logic attempts to compare a wall-clock observation directly with a logical event position
- WHEN time validation runs
- THEN validation denies with a domain-mismatch diagnostic.

### Requirement: Timers are bounded and generation-fenced
r[molten.fabric_time.timers] Aspen MUST expose canonical operations for scheduling, cancelling, and inspecting one-shot and periodic timers. Timer ids MUST bind service generation, time domain, deadline or period, ordering key, coalescing policy, lateness policy, resource charge, and terminal state. Timer firing, cancellation, skipped periods, overload, and cleanup MUST be explicit events.

#### Scenario: One-shot timer fires once
- GIVEN an active generation schedules a one-shot timer within its resource envelope
- WHEN its admitted time boundary is reached
- THEN exactly one firing event is eligible for delivery before the timer enters terminal state.

#### Scenario: Stale timer cannot wake replacement
- GIVEN a service generation is drained and replaced
- WHEN a timer owned by the old generation reaches its deadline
- THEN the event is discarded or reported to cleanup according to policy
- AND it is not delivered to the replacement generation.

### Requirement: The runnable scheduler is explicit and bounded
r[molten.fabric_time.scheduler] Aspen MUST define canonical runnable, wake, yield, block, cancel, complete, and choose transitions for extension callbacks and tasks. Scheduler profiles MUST declare concurrency, queue, fairness or starvation bounds where claimed, ordering, and replay behavior. Deterministic profiles MUST make each nondeterministic runnable choice an explicit replay position.

#### Scenario: Deterministic scheduler replays choices
- GIVEN a run has a recorded runnable-choice sequence and matching initial state
- WHEN the deterministic scheduler replays it
- THEN the same runnable ids are selected at the same canonical choice positions until a divergence is detected.

#### Scenario: Divergent replay fails visibly
- GIVEN replay expects a runnable id that is not eligible
- WHEN that choice position is reached
- THEN replay stops with a divergence diagnostic containing expected and eligible refs
- AND does not silently choose another runnable.

### Requirement: Entropy is provided through admitted streams
r[molten.fabric_time.entropy] Aspen MUST provide bounded canonical entropy operations rather than ambient randomness. Entropy streams MUST bind profile, purpose, service generation, request bounds, and stream position. Deterministic profiles MUST be reproducible from explicit test-run inputs; production cryptographic profiles MUST keep secret material out of receipts and operator readback.

#### Scenario: Deterministic choice repeats
- GIVEN identical deterministic run identity, stream identity, purpose, and request sequence
- WHEN the extension requests bounded choices
- THEN it receives the same choices and stream positions.

#### Scenario: Entropy request exceeds bound
- GIVEN an extension requests more bytes or a wider choice than its admitted profile permits
- WHEN validation runs
- THEN the request denies before entropy is consumed.

### Requirement: Deadlines and leases consume explicit assumptions
r[molten.fabric_time.deadline_lease] Aspen MUST evaluate deadline and lease helpers from declared time domains, uncertainty bounds, owner generation or fencing token, and selected consistency assumptions. A local timer firing or wall-clock observation MUST NOT by itself prove remote expiration, exclusive ownership, safe retry, or absence of stale actors.

#### Scenario: Fenced local lease decision passes
- GIVEN a lease uses a monotonic local deadline, acceptable uncertainty, a current fencing token, and a consistency profile that admits renewal
- WHEN the helper evaluates its inputs
- THEN it returns a bounded local decision and records the assumptions used.

#### Scenario: Missing fencing denies exclusivity claim
- GIVEN an extension asks whether a distributed lease is exclusively held based only on local time
- WHEN lease validation runs
- THEN it denies the exclusivity claim and identifies the missing fencing or consistency evidence.

### Requirement: Live and simulated time preserve one observable contract
r[molten.fabric_time.live_sim_parity] Aspen MUST provide live and deterministic-simulation adapters implementing the same canonical time observations, timer transitions, scheduler events, cancellation, resource bounds, entropy operations, and failure classes. Simulation MAY control virtual time and runnable choice; live execution MUST declare that OS scheduling and wall-clock behavior are not deterministic.

#### Scenario: Timer protocol core runs unchanged
- GIVEN an extension protocol core schedules and handles canonical timer events
- WHEN it runs with live and virtual-time adapters
- THEN the core uses the same command and event types without reading adapter-specific state.

#### Scenario: Clock anomaly is visible
- GIVEN a live wall clock moves backward or outside a declared uncertainty policy
- WHEN the adapter observes the anomaly
- THEN it reports an explicit anomaly outcome or bounded degraded state
- AND does not rewrite prior observations to preserve a false monotonic wall clock.

### Requirement: Time and scheduling evidence is bounded
r[molten.fabric_time.evidence] Aspen MUST emit canonical evidence for profile admission, deterministic run setup, replay traces, material clock anomalies, semantic deadline or lease boundaries selected by an extension, scheduler divergence, and leaked timers or tasks at cleanup. The default production profile MUST NOT require one heavyweight receipt for every clock observation, timer poll, wakeup, or entropy request.

#### Scenario: Deterministic run evidence supports replay
- GIVEN a simulation run completes
- WHEN evidence is exported
- THEN it binds initial state, time profile, scheduler choice trace ref, entropy stream refs or safe seeds, fault plan, and terminal outcome.

#### Scenario: Production entropy remains secret
- GIVEN a production entropy stream serves secret bytes
- WHEN evidence and status are rendered
- THEN they expose only approved profile, purpose, count, and position metadata, never the bytes or secret seed.

### Requirement: Time and scheduling non-claims remain explicit
r[molten.fabric_time.non_claims] Aspen MUST state that time and scheduling ports do not by themselves prove global time, synchronized clocks, distributed lease exclusivity, fairness, liveness, safe retries, absence of partitions, or remote deadline agreement.

#### Scenario: Timer firing is scoped locally
- GIVEN a local lease-expiry timer fires
- WHEN its event is delivered
- THEN the event claims only the selected local time boundary
- AND a higher-level fenced consistency protocol remains responsible for ownership changes.

### Requirement: Time and scheduling validation covers success and failure
r[molten.fabric_time.final_validation] Aspen MUST include positive and negative tests for time domains, checked arithmetic, timer ordering, cancellation, periodic coalescing, stale generations, virtual-time advancement, deterministic scheduler replay, divergent replay, entropy bounds, clock anomalies, overload, deadline and lease assumptions, cleanup, and adapter conformance.

#### Scenario: Shared profile fixtures pass
- GIVEN live and deterministic adapters declare supported profiles
- WHEN shared conformance runs
- THEN both satisfy the canonical operations and failure outcomes for those profiles.

#### Scenario: Fire-after-cancel implementation fails
- GIVEN an adapter delivers an ordinary timer firing after canonical cancellation completed
- WHEN conformance runs
- THEN the adapter fails with a timer terminal-state diagnostic.

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
