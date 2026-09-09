# F10 bounded Yield admission

## ADDED Requirements

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
