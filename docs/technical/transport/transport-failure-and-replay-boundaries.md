# Transport Failure and Replay Boundaries

Failure classification is useful only if it preserves what is known about delivery. A timeout after submission is not evidence that nothing arrived, and rejecting a repeated local request is not exactly-once application processing. This article traces Molten's delivery outcomes and request-consumption boundaries. Read the [fabric transport runtime contract](../../fabric-transport-session-runtime.md) first; the discussion describes existing behavior rather than proposing a retry policy.

## Delivery knowledge and session health are separate axes

The [send transition](../../../crates/molten-core/src/fabric_transport/transition.rs) emits `Pending` when a frame is admitted for submission. A blocked send reports `NotAttempted` and preserves state. An explicit acknowledgement produces `Delivered`. These outcomes name particular transport boundaries, not a durable application transaction.

`fail_session` independently changes the session to `Failed` and resets nonterminal streams. Its delivery classification is conditional: definitive failure reports `NotDelivered`; a nondefinitive failure with bytes in flight reports `Uncertain`; a nondefinitive failure with no in-flight bytes reports `NotAttempted`. Uncertain delivery receives `UnsafeWithoutReconciliation`; the other failure outcomes require higher-level retry policy. There is no implicit retry in this relation.

The distinction prevents a misleading collapse of all error paths into “safe to resend.” It also prevents the converse mistake of treating every failure as uncertain: a definitive refusal can carry stronger negative evidence than a connection lost after submission. The shell is responsible for supplying the observation accurately; the pure relation does not independently inspect network history.

## Where live acknowledgement happens

The [Iroh loopback adapter](../../../src/fabric_transport/parts/adapters/p001/body.rs) records the send transition before running its diagnostic exchange. Network failure causes a nondefinitive failed-session transition. A mismatched echo similarly records malformed-input failure. Only after the expected echo is checked does the adapter submit `AcknowledgeFrame`.

This loopback is a same-process diagnostic rail. Its inspected I/O uses bounded `read_to_end`, whereas the [cross-process shell](../../../src/fabric_transport/cross_process/iroh/parts/shell/p002/body.rs) implements a length-prefixed bounded frame reader. The runtime document's broad description of live-rail length-prefix checking should therefore not be read as a statement that these two implementations share identical framing. This article confines prefix-before-allocation claims to the cross-process reader and leaves the broader prose/implementation mismatch unresolved.

For cross-process effects, the [registered port](../../../src/fabric_transport/cross_process/effect/parts/port/p000/body.rs) records submission before exchanging bytes, checks returned exchange evidence against the request and payload, and then acknowledges. Adapter failure or mismatched exchange evidence returns a canonical failed-session transition reference. A successful function return from this routing layer can therefore contain failure evidence; consumers must interpret the referenced transition, not infer success from the absence of a language-level error.

## Replay denial has a concrete scope

`RegisteredCrossProcessTransportEffectPort` maintains `routed_requests`. It rejects a previously routed request reference before execution. After its control port successfully evaluates the registered command, it inserts the request reference into this set. For a backpressured send, it removes the queued payload and returns the backpressure transition; that request has nevertheless crossed this local consumption boundary. Attempting the identical reference again is not an implicit wait-for-credit mechanism.

This is an in-memory shell property. It does not establish a durable, fleet-wide deduplication registry, survive arbitrary process replacement by itself, or reconcile an application effect that may already have occurred remotely. A different request reference does not make re-executing an application action safe.

There is an important implementation qualification to the governing document's statement that registered requests are consumed once. The [generic registered transport shell](../../../src/fabric_transport/shell.rs) retrieves commands from its request map and does not remove them or maintain a routed-request set in `execute_effect`. The inspected cross-process wrapper supplies explicit replay denial; the generic shell alone does not establish the same property. No code or governing prose is changed here, and no universal single-use guarantee is inferred.

## Worked reasoning: acknowledgement lost after receipt

Suppose an illustrative 600-byte request is submitted, reaches the peer, and the connection disappears before the sender receives the expected acknowledgement. The sender has outstanding in-flight bytes and a nondefinitive disconnect observation. The correct transport conclusion is uncertain delivery. It is not “remote application did nothing,” even if the sender sees only an error.

Resending could duplicate a higher-level action; not resending could leave an intended action uncompleted. Transport cannot resolve that ambiguity. The consuming protocol needs its own evidenced reconciliation semantics if it wants a stronger result. The base port intentionally does not select those semantics on the consumer's behalf.

If the request went through the cross-process registered port, trying the same request reference again is additionally rejected locally. That prevents replay through that port instance, but says nothing about whether the receiver applied the original action once, twice through another channel, or not at all. Local replay control and application effect multiplicity answer different questions.

## Verification and limits

Suggested review should distinguish failure before submission, failure with outstanding bytes, acknowledgement, malformed response, and repeated request routing. The existing [uncertain-disconnect test](../../../crates/molten-core/src/fabric_transport/tests.rs) checks `Uncertain`, reconciliation-required retry disposition, and zero automatic retries. These are source observations; no runtime verification was executed for this article.

The runtime document's parent-run and offline-verification artifacts provide bounded same-host, distinct-process evidence for exact inputs. Logs remain diagnostics, not passing receipts; parent failure artifacts remain non-pass. None of these artifacts proves durable delivery, global ordering, exactly-once behavior, membership, application authority, WAN reliability, or production readiness. The [peer session relation](../../peer-session-transition-relation.md) reinforces that connection facts remain evidence-only until independent gates pass.

## Sources

- [Technical companion](../README.md)
- [Fabric transport session runtime](../../fabric-transport-session-runtime.md)
- [Peer session transition relation](../../peer-session-transition-relation.md)
- [Delivery and failure transition laws](../../../crates/molten-core/src/fabric_transport/transition.rs)
- [Live diagnostic loopback acknowledgement](../../../src/fabric_transport/parts/adapters/p001/body.rs)
- [Cross-process bounded framing](../../../src/fabric_transport/cross_process/iroh/parts/shell/p002/body.rs)
- [Cross-process replay and evidence checks](../../../src/fabric_transport/cross_process/effect/parts/port/p000/body.rs)
- [Generic registered transport shell](../../../src/fabric_transport/shell.rs)
- [Transport failure tests](../../../crates/molten-core/src/fabric_transport/tests.rs)
