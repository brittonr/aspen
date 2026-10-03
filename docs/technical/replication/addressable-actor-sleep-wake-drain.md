# Addressable Actor Sleep, Wake, and Drain

An addressable actor keeps a durable identity while runtime resources can disappear and later be recreated. This article assumes generation fencing, coordination-delivery tokens, and the distinction between a planned transition and executed effects. The [addressable actor runtime profile](../../addressable-actor-runtime.md) is authoritative; this companion explains the inspected lifecycle implementation, not a new actor framework. Return to the [Technical companion](../README.md) for related mechanisms.

## Addressability is not runtime survival

The profile composes existing placement, system-extension, delivery, durable-state, time, resource, authority, and evidence mechanisms. It does not introduce another mailbox or scheduler. An actor key names the durable subject; its placement, generation, and lifecycle sequence determine which requests can currently act on that subject.

The lifecycle distinguishes `dormant`, `starting`, `running`, `draining`, `stopped`, `degraded`, and `recovering`. In particular, dormant is not stopped. Dormancy permits an admitted wake; stopped does not accept another wake under the profile. The generic system-extension lifecycle remains a separate observation that must match the actor profile's mapping. A live process is not sufficient evidence that the corresponding actor generation is running lawfully.

The governing survival matrix is deliberately closed. Durable state, admitted mailbox entries, completed semantic events, and selected checkpoints are durable classes. Processes, streams, and sessions are runtime-only; partial callbacks and in-flight deltas are unsupported. A checkpoint reference identifies selected recovery material, but the reference itself does not demonstrate that any particular class was restored.

## Wake stages expose intermediate state

The [wake planner](../../../crates/molten-core/src/addressable_actor/transition/operations/wake.rs) moves a dormant actor to starting and records an active wake reference. Its ordered intents restore a selected checkpoint if present, start the runtime, and dispatch the selected wake reason. A wake of an already running actor instead plans dispatch without another runtime start.

`start_succeeded` accepts only a starting actor and the matching active wake reference. It then enters running, clears the active wake, and updates the last activity tick. This callback boundary prevents an unrelated completion from being interpreted as the success of the current startup. Request-level generation and lifecycle fencing remain necessary around that local wake check.

The [service shell](../../../src/addressable_actor/parts/service/p000/body.rs) validates profile and host binding, checks expected published state, commits the planned transition, and only then executes effects when the commit is confirmed. Every effect receives a fresh admission observation. Mismatched or refused admission stops the sequence; an execution observation naming another effect or admission is treated as failure. Thus a committed starting state does not imply that checkpoint restoration, runtime startup, and dispatch all succeeded.

## Sleep is quiescence at admitted logical time

Idle sleep is available only from running. The planner rejects nonzero pending mailbox items or unresolved effects. It computes the idle deadline using checked addition of the last activity tick and profile threshold, and rejects a request earlier than that deadline. “Exact idle threshold” means a fixed admitted threshold, not a requirement that the request arrive at one unique tick: the implementation admits the boundary and later logical ticks when other conditions hold.

Accepted sleep records the checkpoint reference, enters dormant, clears the active wake reference, and plans `PersistCheckpoint` before `StopRuntime`. This is a planned state and ordered effect list, not proof that storage and process teardown are atomic. Review of an actual outcome must retain the distinction between planned state, effect observations, and final state.

A new connection can supply a wake reason, but it creates a new runtime connection. The profile does not resume an old stream merely because the same actor key is addressable again.

## Drain is a different lifecycle commitment

The [lifecycle planner](../../../crates/molten-core/src/addressable_actor/transition/operations/lifecycle.rs) permits beginning drain from running and enters draining without immediate runtime-stop effects. Under the governing contract, draining blocks new work while bounded current work completes. `drain_succeeded` requires the draining phase and zero remaining items; a nonzero count is not an acceptable approximate completion.

Successful drain records a checkpoint, enters stopped, and orders checkpoint persistence before runtime stop. Sleep returns an actor to an addressable dormant state, whereas completed drain closes this profile's wake path. Treating drain as merely a longer idle sleep would erase this distinction and make shutdown callbacks ambiguous.

## Worked sleep-and-rewake scenario

Suppose, illustratively, an actor last handled activity at logical tick 40 and has an idle threshold of ten ticks. At tick 49, sleep is too early. At tick 50, one pending mailbox item still blocks sleep. After that work completes and the supplied quiescence facts are clear, a sufficiently late admitted sleep can select checkpoint `C` and plan checkpoint publication followed by runtime stop.

A subsequent message wake uses the same actor identity but must carry current placement, generation, and lifecycle expectations. Restoration of `C` precedes runtime start in the plan. A delayed startup callback from a previous wake is not a substitute for the current active wake reference. Even if restoration succeeds, a policy change before the next effect can deny runtime start through fresh admission.

Delivery completion has another boundary: the actor supplies a durable semantic-event commit reference before the profile plans acknowledgement. The [delivery integration](../../../crates/molten-core/src/addressable_actor/transition/operations/delivery.rs) records completed event information and plans the acknowledgement effect. This does not turn coordination delivery into exactly-once execution of external effects.

## Unknown effects and evidence limits

An executed effect with unknown outcome stops the remaining sequence. The shell attempts a separate transition recording the unknown effect and degraded state. A narrow persistence qualification matters: [recording logic](../../../src/addressable_actor/parts/service/p001/body.rs) returns the new published state only when that follow-up commit is confirmed; otherwise it retains the previous value. The governing profile says unknown effects are recorded and move the actor to degraded. The inspected helper does not establish durable recording when its own commit remains unconfirmed. This companion leaves that boundary explicit rather than assuming persistence succeeded.

Under the governing contract, recorded unknown state blocks wake, delivery, and recovery until explicit resolution permits checkpoint recovery; resolution does not authorize automatic retry of the uncertain effect. Suggested review should inspect stale wake callbacks, sleep at the threshold with pending work, nonzero drain remainder, admission changes between effects, and unknown-effect recording with an unconfirmed commit. These are proposed checks, not executed results here.

Canonical receipts bind planned and final state plus effect and status observations. They do not grant retry, mutation, activation, release, or production authority. Neither sleeping nor checkpoint recovery proves that runtime-only resources survived.

## Sources

- [Addressable actor runtime profile](../../addressable-actor-runtime.md)
- [Coordination delivery extension](../../coordination-delivery.md)
- [Wake and idle-sleep transitions](../../../crates/molten-core/src/addressable_actor/transition/operations/wake.rs)
- [Drain and stop transitions](../../../crates/molten-core/src/addressable_actor/transition/operations/lifecycle.rs)
- [Semantic completion and unknown-effect transitions](../../../crates/molten-core/src/addressable_actor/transition/operations/delivery.rs)
- [Actor shell admission and effect execution](../../../src/addressable_actor/parts/service/p000/body.rs)
- [Actor reconciliation, unknown recording, and receipts](../../../src/addressable_actor/parts/service/p001/body.rs)
- [Technical companion](../README.md)
