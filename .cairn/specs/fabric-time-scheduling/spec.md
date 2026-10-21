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

### Requirement: Exponential growth saturates without high-bit loss

r[molten.audit_f12.saturation]
Molten MUST compute exponential retry delay with checked multiplication or a pre-shift overflow check and cap it at `maximum_delay_ticks`.

#### Scenario: Ordinary growth remains exact
- GIVEN an admitted exponential policy whose mathematical delay is below its maximum
- WHEN the generic planner computes a retry
- THEN the delay equals the mathematical exponential result before admitted jitter

#### Scenario: High bits exceed the integer range
- GIVEN base delay two, attempt 63, maximum delay 128, no jitter, and an admitted attempt budget
- WHEN the generic planner computes a retry
- THEN the delay is 128 rather than zero

### Requirement: Arithmetic bounds remain explicit

r[molten.audit_f12.bounds]
Molten MUST use named arithmetic bounds and handle every admitted attempt without narrowing wrap or an attempt-sized loop.
Molten MUST preserve finite-attempt admission and checked deadline arithmetic.

#### Scenario: A large admitted attempt saturates
- GIVEN an admitted attempt exceeds the delay integer width and the target instant can represent the capped delay
- WHEN the exponential planner runs
- THEN it returns the maximum delay without narrowing-conversion failure

#### Scenario: A deadline cannot represent the delay
- GIVEN the capped delay exceeds the remaining target-instant range
- WHEN deadline construction runs
- THEN it returns the existing arithmetic error without a wrapped deadline or retry effect

### Requirement: Existing admission and fixed behavior remain intact

r[molten.audit_f12.compatibility]
Molten MUST preserve fixed-delay outputs and generation, domain, attempt, and jitter admission.
Shells MUST preserve consumer state and schedule no retry after a rejected plan.

#### Scenario: The current delivery profile remains fixed
- GIVEN the coordination-delivery fixed-delay profile and valid explicit time inputs
- WHEN it requests a retry
- THEN its delay and deadline remain unchanged by the exponential correction

#### Scenario: Retry inputs are invalid
- GIVEN exhausted attempts, invalid jitter, a stale generation, or a mismatched time domain
- WHEN retry admission runs
- THEN the declared error occurs without a timer effect or consumer-state mutation

### Requirement: Evidence distinguishes planner behavior from deployment claims

r[molten.audit_f12.validation]
Molten MUST retain positive and negative normal repository tests for arithmetic, adapters, and replay.
Evidence MUST scope F12 to the generic exponential planner and MUST NOT claim fixed-profile exposure, safe retry, or measured performance.

#### Scenario: Corrected retry evidence matches the plan
- GIVEN a valid saturated plan and matching adapter inputs
- WHEN the shell records the retry observation
- THEN its canonical delay and deadline match the corrected plan

#### Scenario: Historical replay expects wrapped delay
- GIVEN a legacy trace expects the former truncated exponential result
- WHEN corrected replay evaluates that position
- THEN it reports divergence or an explicit unsupported cohort without rewriting historical receipts

### Requirement: Blocked Wake resumes the same occurrence

r[molten.audit_f09.resume]
Molten MUST admit current-generation Wake for an existing Blocked occurrence without another runnable record or active-slot charge.

#### Scenario: Full active capacity still permits resume
- GIVEN one blocked occurrence occupies the admitted active limit and the ready queue has space
- WHEN Wake names that occurrence
- THEN the phase becomes Ready and the active count stays unchanged

#### Scenario: Another phase remains a duplicate
- GIVEN the occurrence is Ready, Running, Completed, or Cancelled
- WHEN Wake names that occurrence
- THEN duplicate admission denies without state mutation

### Requirement: Resume shares bounded ready admission

r[molten.audit_f09.queue]
Molten MUST apply `max_scheduler_queue_depth` and the admitted overload policy before blocked resume or new Wake enters Ready.

#### Scenario: Queue capacity exists
- GIVEN a blocked occurrence and a free ready slot
- WHEN Wake passes admission
- THEN exactly one ready entry represents the occurrence

#### Scenario: Queue capacity is exhausted
- GIVEN the ready count equals `max_scheduler_queue_depth`
- WHEN Wake requests blocked resume
- THEN Reject or Backpressure preserves phases, counts, priorities, and sequence positions

### Requirement: Resume preserves explicit ordering and fences

r[molten.audit_f09.ordering]
Molten MUST assign a checked fresh enqueue sequence, apply the requested priority, and reset ready wait accounting after successful resume.
Molten MUST preserve state on arithmetic error and discard stale-generation commands without effects.

#### Scenario: Resume enters FIFO order
- GIVEN older ready work and a blocked current occurrence
- WHEN Wake resumes that occurrence
- THEN its new enqueue position follows older ready work under FIFO policy

#### Scenario: A sequence cannot advance
- GIVEN the enqueue sequence is exhausted
- WHEN Wake requests resume
- THEN an overflow result preserves all state and prevents wake effects

### Requirement: Adapter and evidence claims remain scoped

r[molten.audit_f09.validation]
Molten MUST cover core and shell resume behavior with positive and negative normal repository tests.
Molten MUST distinguish executed F09 evidence from static shell analysis and MUST NOT claim global liveness or measured performance.

#### Scenario: The extension resumes at its active limit
- GIVEN matching service and profile bindings with queue space and a full active envelope
- WHEN the extension submits Wake for its blocked occurrence
- THEN adapter conformance records the admitted resume without another active charge

#### Scenario: A stale callback requests resume
- GIVEN a callback belongs to a retired generation
- WHEN its Wake reaches the adapter
- THEN no current-generation wake effect occurs and evidence records the scoped denial or discard

### Requirement: Yield cannot exceed the ready bound

r[molten.audit_f10.queue]
Molten MUST admit Yield against `max_scheduler_queue_depth` before a Running occurrence enters Ready.

#### Scenario: Yield has ready capacity
- GIVEN A is Running and a ready slot is free
- WHEN A yields
- THEN A becomes Ready without exceeding the queue bound

#### Scenario: Yield encounters a full ready queue
- GIVEN queue bound one, A Running, and B Ready
- WHEN A yields
- THEN the admitted overload policy returns Reject or Backpressure and A remains Running

### Requirement: Every Ready transition shares admission

r[molten.audit_f10.shared_admission]
Molten MUST use one ready-capacity contract for new Wake, blocked Wake, and Yield.
Molten MUST charge an active slot only for a new occurrence.

#### Scenario: Existing work enters Ready
- GIVEN active capacity is full but ready capacity is free
- WHEN an existing blocked occurrence wakes or a Running occurrence yields
- THEN no additional active slot is necessary

#### Scenario: New work exceeds the active bound
- GIVEN active capacity is full and ready capacity is free
- WHEN Wake names a new occurrence
- THEN admission rejects or applies backpressure without a new record

### Requirement: Denied Yield preserves state and effects

r[molten.audit_f10.atomicity]
Molten MUST preserve all state on denied Yield and use checked counts and enqueue arithmetic before successful mutation.
The shell MUST emit no accepted-yield effect or receipt for a denied transition.

#### Scenario: Successful Yield receives a fresh position
- GIVEN a valid Running occurrence and sufficient queue capacity
- WHEN Yield passes checked arithmetic
- THEN the occurrence receives a fresh enqueue position and the shell observes `Yielded`

#### Scenario: Invalid or overflowing Yield does not mutate
- GIVEN an invalid source phase or exhausted enqueue sequence
- WHEN Yield requests a transition
- THEN the typed error preserves phases, counts, reservations, and sequence positions

### Requirement: Queue evidence is reproducible and bounded

r[molten.audit_f10.validation]
Molten MUST retain positive and negative core, capacity, adapter, and replay tests in normal repository paths.
Evidence MUST distinguish the executed F10 counterexample from unexecuted integration claims and MUST NOT claim global liveness or measured performance.

#### Scenario: Adapters agree on overload
- GIVEN equivalent live and simulation inputs with a full ready queue
- WHEN both adapters process Yield
- THEN both report the admitted overload result without an accepted-yield effect

#### Scenario: Historical replay contains over-capacity Yield
- GIVEN a recorded choice trace relies on the former queue-bound violation
- WHEN corrected replay evaluates that trace
- THEN replay reports divergence or an explicit unsupported cohort instead of silently changing history

### Requirement: Terminal state has a finite named bound

r[molten.audit_f11.retention]
Molten MUST bound terminal records through a named limit derived from the admitted capacity contract.
Molten MUST reclaim terminal records deterministically without reclaiming active occurrences or widening initialized capacity.

#### Scenario: Repeated completion remains bounded
- GIVEN active limit one and an admitted finite terminal-retention bound
- WHEN successive fresh occurrences wake, run, and complete beyond that retention bound
- THEN retained terminal records stay within the bound and active work remains admissible within physical capacity

#### Scenario: Invalid retention capacity denies activation
- GIVEN the retention plan is unrepresentable or exceeds admitted physical capacity
- WHEN capacity admission runs
- THEN activation denies without a smaller hidden scheduler or partial state

### Requirement: Eviction cannot permit ABA resurrection

r[molten.audit_f11.freshness]
Molten MUST bind each occurrence to an explicit fresh issuance sequence and generation fence independent of retained tombstones.
Molten MUST reject old occurrence admission after eviction without an unbounded retired-identity set.

#### Scenario: A fresh occurrence reuses a logical name
- GIVEN an old terminal occurrence is evicted and a fresh issuance sequence exceeds the generation high-water fence
- WHEN admission names the same logical runnable with the fresh occurrence identity
- THEN only the fresh occurrence becomes eligible

#### Scenario: An old callback arrives after eviction
- GIVEN an old occurrence is evicted and its logical name now identifies different work
- WHEN its Wake, Complete, Yield, Block, or Cancel callback arrives
- THEN the old callback cannot create, resume, complete, or mutate the fresh occurrence

### Requirement: Retention arithmetic and rejection are atomic

r[molten.audit_f11.atomicity]
Molten MUST use checked counts, capacity calculations, and occurrence-sequence advancement.
Molten MUST preserve state on rejected issuance, stale input, arithmetic error, or invalid transition.

#### Scenario: Cleanup reclaims eligible terminal records
- GIVEN a retired generation contains terminal and active records
- WHEN authorized cleanup cancels its active work and applies retention
- THEN the result respects the named bounds and cannot affect another active generation

#### Scenario: Fresh occurrence issuance is exhausted
- GIVEN the occurrence sequence cannot advance
- WHEN a new occurrence requests admission
- THEN admission returns an explicit error without wrap, generation fallback, eviction side effects, or callback execution

### Requirement: Replay and evidence preserve occurrence meaning

r[molten.audit_f11.validation]
Molten MUST cover retention, eviction, callback fences, adapters, and replay with positive and negative normal repository tests.
Molten MUST distinguish executed F11 accumulation evidence from unexecuted fence claims and reject incompatible legacy identity interpretation explicitly.

#### Scenario: Matching replay preserves reclamation
- GIVEN identical admitted occurrence inputs, profile, and initial state
- WHEN replay runs across terminal eviction
- THEN it reproduces occurrence selection and deterministic reclamation without unbounded history

#### Scenario: A legacy callback lacks a fence
- GIVEN a callback cannot identify its occurrence under the admitted compatibility contract
- WHEN an adapter decodes it
- THEN admission denies without mapping it to current work or inventing historical receipts
