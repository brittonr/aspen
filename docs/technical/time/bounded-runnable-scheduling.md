# Bounded runnable scheduling

Molten's scheduler separates admission of work, readiness, selection, and host wake effects. This [Technical companion](../README.md) explains why bounds apply to transitions rather than just initial insertion. Read the [fabric-time runtime](../../fabric-time-scheduler-runtime.md) first; the discussion assumes generation-bound identities and an admitted scheduler policy. It describes local in-memory laws and capacity observations, not a general-purpose operating-system scheduling guarantee.

## Three envelopes, not one queue size

`RunnableKey` identifies a service, generation, and runnable occurrence. `RunnableState` records priority, enqueue sequence, wait turns, and one of five phases: ready, running, blocked, completed, or cancelled. `SchedulerState` carries the profile reference, generation, next enqueue sequence, choice sequence, and retained runnable states ([scheduler core](../../../crates/molten-core/src/fabric_time/scheduler/mod.rs)).

The relevant logical envelopes are distinct:

- Active work includes nonterminal runnable occurrences and is bounded by `max_runnables` when admitting new work.
- Ready work consumes `max_scheduler_queue_depth`.
- Selected running work consumes `max_scheduler_concurrency`.

An occurrence can stop consuming ready capacity without ceasing to be active. A blocked occurrence retains its active slot; so does a running occurrence. Conversely, completing an occurrence releases it from the active count but does not, in the inspected core, remove its retained `RunnableState` from the vector. Thus an active-work bound alone is not a proof of bounded lifetime history retention. This is an important scope limit when interpreting the governing description of bounded total runnables.

## Every path back to ready is admission

The helper `ready_overload_action` is shared by new wake, blocked wake, and yield. Queue fullness is checked for all three. Only a genuinely new occurrence is additionally charged against the active-work bound. Under the admitted policy, denied readiness returns either `RejectedOverload` or `Backpressure`, with an unchanged next state.

An existing occurrence can be woken only from blocked. Waking an already ready, running, completed, or cancelled occurrence yields `DuplicateRunnable`; it does not silently revive or reposition it. A successful blocked wake installs the supplied priority, takes a fresh checked enqueue sequence, and resets wait turns. Yield similarly requires running state and available ready capacity, then requeues the occurrence. These mechanics preserve atomicity: an unsuccessful transition does not partially free concurrency, change priority, or consume an enqueue position ([scheduler core](../../../crates/molten-core/src/fabric_time/scheduler/mod.rs)).

`ExtensionTimeContext` adds service identity, admitted port-profile, and extension resource checks. Its blocked-resume path intentionally avoids charging the extension's active envelope twice. This outer boundary does not replace the shared ready-capacity law ([extension shell](../../../src/fabric_time/shell.rs)).

## Worked saturation scenario

Consider an illustrative profile permitting three active occurrences, two ready entries, and one running selection. A is running; B and C are ready. Active usage is three, ready usage two, and running usage one.

If A yields now, the ready queue is full. Under backpressure, the transition leaves A running and leaves every sequence and wait count unchanged. Treating the attempted yield as successful would create an inconsistent state: either three ready entries violate the queue bound or A disappears from running without a valid destination.

Now suppose B blocks. Ready usage falls to one while active usage remains three. A can yield successfully, taking the next enqueue position. B cannot immediately resume because the ready queue again contains C and A. After one of them is selected, B may resume without acquiring a fourth active slot; it already owns its original occurrence. A wake for a new D remains a new-work admission and can still be denied by the active limit even when ready capacity is available.

This reasoning explains why a blanket “reject whenever active count equals the maximum” is wrong for resume, and why checking capacity only at initial wake is wrong for yield.

## Ordering, fairness, and replay are separate checks

Selection first enforces concurrency and gathers ready candidates from the active generation. FIFO orders by enqueue sequence; priority/FIFO orders higher numeric priorities first, then enqueue sequence, with the key as a final tie-breaker. When a finite fairness bound is present, overdue candidates precede ordinary ordering, and longer-waiting overdue work precedes shorter-waiting work.

Deterministic replay optionally checks that a supplied choice equals the computed choice. `RecordedChoiceRequired` instead requires an explicitly recorded eligible candidate. Eligibility alone is not the last check: fairness enforcement can reject a recorded selection that neglects longer-waiting overdue work. Successful selection increments the choice sequence and other ready candidates' wait counters using checked arithmetic.

These are turn-based decisions. A bounded wait counter is not a wall-clock latency bound, and absence of a fairness bound retains the explicit fairness non-claim in the [governing contract](../../fabric-time-scheduler-runtime.md#runnable-scheduler).

## Physical capacity and verification limits

The shell capacity `Runtime` derives a plan from the profile and generation and fallibly reserves runnable and queue storage. Reservation failure denies activation rather than shrinking the scheduler. Accounting and reservation ownership are separate from selection semantics; concrete reservation slot types do not become canonical identity ([capacity shell](../../../src/fabric_time/capacity/mod.rs)). The pure scheduler also clones and collects state, so these reservations are not evidence of whole-runtime allocation freedom.

Suggested review exercises reproduce the saturation scenario, verify unchanged denied transitions, and test stale generation commands separately from stale scheduler state. Review terminal retention separately from active counters. No scheduler run is claimed here. Capacity observations and canonical choices do not establish host fairness, liveness, production readiness, or remote execution. [Runtime limit profiles](../../runtime-limit-profiles.md) select budgets, not additional execution authority.

## Sources

- [Fabric-time scheduler and capacity contract](../../fabric-time-scheduler-runtime.md)
- [Runtime limit profiles](../../runtime-limit-profiles.md)
- [Runnable state machine and choice ordering](../../../crates/molten-core/src/fabric_time/scheduler/mod.rs)
- [System-extension scheduler admission](../../../src/fabric_time/shell.rs)
- [Fallible capacity reservation shell](../../../src/fabric_time/capacity/mod.rs)
- [Technical companion](../README.md)
