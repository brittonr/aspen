# Generation-Fenced Lifecycle

Generation fencing makes delayed work distinguishable from work belonging to the active service incarnation. It does not merely attach a version label to status. This article assumes the lifecycle vocabulary of the [system-extension runtime](../../system-extension-runtime.md), and explains the pure transition relation, callback gates, and shell obligations. It belongs to the [Technical companion](../README.md).

## State, event, and admission are separate

`LifecycleState` records generation, phase, restart attempts, health, and an optional checkpoint reference. `LifecycleEvent` names an event kind, its generation, and narrowly permitted supplementary fields. `plan_lifecycle_transition` accepts this state and event together with resource usage and a restart budget. It returns a proposed state or issues; it performs no process, timer, storage, or network work. See the [lifecycle implementation](../../../crates/molten-core/src/system_extension/lifecycle.rs).

The initial absent state uses generation zero, while installation is checked against the designated initial generation. Other events must name the current generation. This prevents a delayed successful completion from advancing a newer incarnation merely because its event kind would otherwise be legal.

Event shape is checked before the phase/event relation. Only upgrade and rollback beginnings accept `next_generation`, and they require the checked successor of the current generation. Overflow is a denial, not wraparound. Failure events require a failure class; unrelated events cannot carry one. Checkpoint and recovery success require valid checkpoint references. Rejecting surplus fields prevents a caller from smuggling facts into an event whose semantics do not consume them.

## The fence operates at multiple boundaries

A legal lifecycle transition is not sufficient to invoke arbitrary callback code. `plan_callback_dispatch` checks declared callback kind, allowed phase, active generation, event and payload references, explicit deadline, cancellation, and resource admission. Ordinary requests, messages, streams, timers, and health callbacks run only in `Running`. Recovery also has explicit admission in recovery, upgrade, and rollback phases. The [dispatch implementation](../../../crates/molten-core/src/system_extension/dispatch.rs) is the relevant second boundary.

Successful scheduling increments the invocation sequence with checked arithmetic. A non-scheduling resource decision carries no invocation. Thus a queue/backpressure decision is not a callback observation, and an event reference is not evidence that code ran. The shell records actual invocation only after receiving a dispatch plan containing an invocation.

Typed effects carry a generation independently of the callback event. Outcome validation checks them against the invocation generation, and routing checks them again against the active host generation. That second check matters when an approved receipt survives a later cutover. Native effect completion delivery has another explicit generation check, as described in the [native host reference](../../native-system-extension-host.md). These checks protect distinct boundaries rather than relying on one early label indefinitely.

## Draining is a state claim backed by counters

A callback returning successfully does not alone establish drain completion. The lifecycle relation permits `DrainSucceeded` only when resource usage is idle, and applies the same condition to `ShutdownSucceeded`. Idleness covers concurrent callbacks, queued events, in-flight bytes, open streams, timers, and effect requests, not merely the number of executing callbacks. The counters and checked resource operations are defined in [supervision](../../../crates/molten-core/src/system_extension/supervision.rs).

This is a deterministic invariant over tracked usage. Whether an adapter has correctly reflected every live external operation into that usage is an integration question. In particular, native removal adds unresolved-operation and ingress checks; its stronger service-level gate should not be confused with the generic lifecycle's counter gate.

## Worked delayed-completion scenario

Suppose, illustratively, an extension is running in generation 7. A request completes and produces an approved storage effect, but the caller delays routing it. An upgrade begins with event generation 7 and `next_generation` 8. Once the generated state is active, the old effect still says generation 7.

Routing the old callback receipt now fails typed-effect validation even if the receipt is host-owned and the port identifier remains available. The extension's logical identity may be unchanged, but the generation has changed. Similarly, an old timer delivered as a callback event fails dispatch before executor invocation. Neither failure requires decoding application payload semantics.

Now assume the new instance is draining with no executing callbacks but one tracked timer. A successful drain callback cannot force the phase to `Drained`; the transition receives non-idle usage. Clearing that timer through the appropriate accounting path changes the relevant fact. Replaying the same success event without changing usage does not.

## Review and suggested verification

Review each entry point in terms of the generation it accepts, the state against which it compares, and whether the comparison occurs before effects. The existing [core tests](../../../crates/molten-core/src/system_extension/tests.rs) include delayed old-generation work and drain completion with live resources. Suggested additional review cases include a non-sequential successor, generation overflow, a stale effect routed after replacement, and cancellation before invocation. These are proposed verification activities; no runtime results are claimed here.

The [plugin lifecycle FSM](../../plugin-lifecycle-fsm.md) provides a useful comparison for authority closure after removal, but its guard receipts and states are a separate relation. Importing a plugin transition into this lifecycle by analogy would erase rather than strengthen the boundary.

## Limits and non-claims

A generation fence is not global consensus, physical cancellation of an already-issued provider operation, or exactly-once execution. Restart attempts are not inherently generation changes: the inspected restart transition increments the attempt count while preserving the current generation. The guarantee discussed here is rejection at explicit admission boundaries, conditional on the host's active state and supplied facts, not universal revocation of every external side effect.

## Sources

- [System-extension runtime](../../system-extension-runtime.md)
- [Native system-extension host](../../native-system-extension-host.md)
- [Plugin lifecycle FSM](../../plugin-lifecycle-fsm.md)
- [Lifecycle transition relation](../../../crates/molten-core/src/system_extension/lifecycle.rs)
- [Callback and effect validation](../../../crates/molten-core/src/system_extension/dispatch.rs)
- [Resource accounting and supervision](../../../crates/molten-core/src/system_extension/supervision.rs)
- [Core regression cases](../../../crates/molten-core/src/system_extension/tests.rs)
