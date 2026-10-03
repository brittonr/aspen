# Timer generation fencing

A timer is deferred work owned by a particular lifecycle generation, not an instruction that remains valid forever because its deadline has passed. This [Technical companion](../README.md) develops the timer state machine described by the [fabric-time runtime](../../fabric-time-scheduler-runtime.md). It assumes familiarity with admitted profiles and explicit clock domains; its scope is local transition semantics, not distributed fencing of external resources.

## Identity and admission precede expiry

`TimerKey` combines `service_id`, `generation`, and `sequence`. The schedule request also binds a profile, domain, deadline, timer kind, ordering key, coalescing policy, lateness policy, overload policy, and resource charge. In `schedule_timer`, a zero generation is rejected, and the request generation must match the explicitly supplied active generation. The function checks profile equality, supported domain membership, nonzero resource charges, checked slot accounting, positive periodic intervals, and positive catch-up bounds ([timer implementation](../../../crates/molten-core/src/fabric_time/timer.rs)).

These checks have different responsibilities. The service identifier's shape is checked by the core, while correspondence with the actual hosting service is checked by `ExtensionTimeContext::schedule_timer`. That shell also requires a bound timer profile and checks the host's timer envelope before invoking the core ([extension time shell](../../../src/fabric_time/shell.rs)). A syntactically valid service name is consequently not sufficient admission authority.

The core accepts explicit state and counts. It does not discover a lifecycle transition by querying a host, nor does it maintain a hidden global timer registry. In particular, generation comparison is only as current as the active-generation argument supplied by the integrating shell. Canonical evidence about an earlier scheduling decision cannot substitute for that current input.

## Polling is a fenced transition

`poll_timer` first rejects an already terminal timer. For a scheduled timer whose key belongs to another generation, it returns a cancelled next state with `DiscardedStaleGeneration` and zero deliveries. This check occurs before due-time arithmetic. An overdue timer is therefore not entitled to run merely because its deadline preceded a restart.

For a current scheduled timer, a future deadline produces `NotDue`. Otherwise the core computes lateness and periods due, applies lateness policy, computes the delivery plan and queue charge, and finally applies overload policy if capacity is insufficient. The returned next state and action are a pure decision. They are not an operating-system callback or evidence that application work completed.

Cancellation and cleanup have deliberately different interfaces. `cancel_timer` requires a matching active generation and a scheduled phase. `cleanup_generation` traverses supplied states and cancels scheduled timers belonging to the retired generation. A completed one-shot remains completed, and unrelated generations remain unchanged. `order_due_timers` excludes stale and terminal states, ordering eligible timers by deadline, ordering key, and full timer key before truncating to its explicit maximum ([timer implementation](../../../crates/molten-core/src/fabric_time/timer.rs)).

## Periodic time does not drift with polling

The periodic calculation advances from the prior scheduled deadline, not from the poll observation. For deadline D, period P, and observed time N at or after D, the due count is `(N − D) / P + 1`. The next deadline adds that count times P to D with checked arithmetic.

Consider an illustrative timer in generation 7 with deadline 100 and period 10. It is polled at 135, so four periods are due and the next deadline is 140. With `DeliverEach { max_catch_up: 2 }`, two deliveries are planned and two periods skipped. `CoalesceLatest` plans one delivery, three skipped periods, and a coalesced action. `SkipMissed` also plans one delivery and three skipped periods, but reports `Deliver` rather than `Coalesced`. The action vocabulary therefore preserves a policy distinction even where counts agree.

If the active generation is instead 8, none of those calculations authorize delivery: the stale-generation transition cancels the timer. This is the key fencing property at the local state-machine boundary. It does not imply that some previously emitted external callback can be recalled.

## Overload is not lateness

Suppose the same four-period timer requires two queue units per delivery and its catch-up policy requests two deliveries. Required capacity is four units. With only three available, `RejectAndRetain` and `Backpressure` preserve state and do not advance the deadline. `DropDue` advances without delivery and records the due periods as skipped. A lateness-policy rejection instead yields `DroppedLate`, and is evaluated before queue capacity.

These distinctions matter operationally. Retaining work can expose the same overdue state on a later poll; dropping it consumes that schedule opportunity. Neither behavior should be summarized as “the timer fired late.” The [runtime limit profile overview](../../runtime-limit-profiles.md) likewise distinguishes admitted budgets from authority: enlarging a budget cannot make a retired generation current.

## Verification and non-claims

Suggested review cases cross generation, phase, lateness, and capacity: poll a stale scheduled timer, repoll a completed one-shot, cancel after cancellation, clean up a mixed-generation set, and compare retain with drop under identical overload. For periodic arithmetic, include a deadline near the integer ceiling and a catch-up interval spanning several periods. These are suggested checks, not executed evidence.

The governing fixture command can exercise live and simulated adapters against common timer laws. Its observations do not establish callback delivery, persistence, exactly-once application effects, synchronized clocks, or distributed lease exclusivity. Stable local ordering is not a global ordering service, and a generation field is not by itself a remote storage fencing token.

## Sources

- [Fabric-time timer and lifecycle contract](../../fabric-time-scheduler-runtime.md)
- [Runtime limit profiles](../../runtime-limit-profiles.md)
- [Timer admission, polling, ordering, and cleanup](../../../crates/molten-core/src/fabric_time/timer.rs)
- [System-extension timer admission](../../../src/fabric_time/shell.rs)
- [Technical companion](../README.md)
