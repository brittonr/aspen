# Sans-I/O Core and Shell Drain

A transport state transition is not a socket operation, and marking a listener closed is not the same observation as awaiting endpoint shutdown. Molten keeps these responsibilities separate through a pure transport core and effectful adapter shells. This article explains that separation and its consequences for readiness, draining, cleanup evidence, and review. The governing reference is the [fabric transport runtime contract](../../fabric-transport-session-runtime.md).

## What the core can decide

The [pure module boundary](../../../crates/molten-core/src/fabric_transport/mod.rs) explicitly excludes sockets, executors, clocks, randomness, and simulator runtimes. `apply_transport_command` receives a profile, prior state, and command, then returns a transition or validation issues. Observed ticks and failure classes enter as data. This permits deterministic reasoning about the same explicit input state without pretending to model every network mechanism.

The core can determine that a scoped handle belongs to the wrong generation, that a deadline has been reached, or that a session prevents listener cleanup. It cannot dial an endpoint, verify that a process actually terminated, or discover whether a remote application durably committed a message. Those observations require shell mechanisms and, where applicable, additional evidence contracts.

Both the deterministic adapter and the Iroh adapter invoke this command algebra and canonicalize its result before installing the next state. The [adapter implementation](../../../src/fabric_transport/parts/adapters/p000/body.rs) also exposes explicit fault injection in the deterministic shell. That establishes a shared transition vocabulary; it does not establish that simulation reproduces every timing, buffering, or failure mode of live Iroh.

## Readiness is composed before publication

The [cross-process listener shell](../../../src/fabric_transport/cross_process/iroh/parts/shell/p001/body.rs) first validates its inputs and binds an explicit endpoint. It derives endpoint identity and locator information, constructs the canonical descriptor, plans listener state, starts the listener, and submits a readiness observation. That observation distinguishes endpoint setup, exact ALPN activation, registration ownership, transport capability, protocol capability, and active profile.

Only after these steps does the shell plan endpoint export. Publication is thus more than serializing an address. The descriptor is bound to the admitted profile and protocol and has a lifecycle context. An address that happens to accept a connection is not a substitute for the expected endpoint binding.

As the [ALPN contract](../../iroh-alpn-routing-registry.md) emphasizes, routing readiness remains transport evidence. A ready listener does not grant a peer node-control authority or satisfy application-specific resource and policy gates.

## There are two drain levels

In the [base transport relation](../../../crates/molten-core/src/fabric_transport/transition.rs), `BeginDrain` marks the registration draining and changes matching live sessions to the draining phase. Opening a new session then denies because the registration is no longer active. `CleanupListener` requires the correct scope, a cleanup evidence reference, draining registration state, and no nonterminal session in that ALPN/generation cohort. On success, it removes the registration.

The cross-process listener has a separate lifecycle state and a real endpoint. Its `drain_and_close` method begins drain, checks that `active_sessions` is zero, applies the close command, awaits `endpoint.close()`, begins cleanup, constructs cleanup evidence, and completes cleanup. Importantly, this method rejects an active-session count rather than autonomously waiting for or cancelling all sessions. The runtime contract's requirement that active sessions reach terminal state is a precondition to successful closure here, not a hidden background draining scheduler.

These two levels should remain distinguishable in evidence review. A pure cleanup transition establishes that the supplied state and evidence passed the relation. The successful shell path additionally places endpoint-close completion before cleanup completion. Neither fact alone establishes cleanup of unrelated processes or resources outside that listener's ownership scope.

## Worked reasoning: drain with one pending session

Consider an illustrative generation-5 listener with one established session. An owner requests drain while that session still has outstanding work. The base drain transition changes admission state, so a later attempt to open another generation-5 session denies. Premature base cleanup also denies because a nonterminal session remains.

The owner must now reach a legitimate terminal session state through the relevant close, cancellation, or failure path. This does not imply successful delivery of pending work: a failed session can be terminal while its delivery outcome remains uncertain. Terminal lifecycle state and acknowledged delivery are different facts.

At the shell level, calling `drain_and_close` while `active_sessions` remains nonzero returns an error. After the listener's session accounting reaches zero, the successful close path can await the endpoint and construct cleanup evidence. The example does not assume that a cleanup reference can be invented to bypass live-session accounting, nor that dropping a runtime object is equivalent to this successful evidence path.

## Verification and limits

Suggested review follows ordering rather than just final flags: readiness before export; drain before refusal of new sessions; terminal session accounting before close; awaited endpoint close before cleanup completion. The existing [transport tests](../../../crates/molten-core/src/fabric_transport/tests.rs) exercise refusal of new sessions during drain and denial of premature cleanup. They were inspected, not run for this article.

For live evidence, the runtime document describes the distinct-process runner and offline verifier. A useful future exercise is to inspect both child terminal artifacts and parent-observed process evidence, rather than treating a same-process loopback as distributed proof. This is suggested verification, not an execution report.

No automatic retry, consensus, durable delivery, or exactly-once semantics emerge from a sans-I/O architecture. Purity makes the specified relation inspectable; it does not make ambient mechanisms deterministic, grant authority, or qualify a deployment for production.

## Sources

- [Technical companion](../README.md)
- [Fabric transport session runtime](../../fabric-transport-session-runtime.md)
- [Iroh ALPN routing registry](../../iroh-alpn-routing-registry.md)
- [Pure transport module boundary](../../../crates/molten-core/src/fabric_transport/mod.rs)
- [Drain and cleanup transition laws](../../../crates/molten-core/src/fabric_transport/transition.rs)
- [Deterministic adapter and fault observations](../../../src/fabric_transport/parts/adapters/p000/body.rs)
- [Cross-process listener readiness and close](../../../src/fabric_transport/cross_process/iroh/parts/shell/p001/body.rs)
- [Transport drain tests](../../../crates/molten-core/src/fabric_transport/tests.rs)
