# Backpressure and Resource Bounds

A bounded transport distinguishes a valid operation that cannot progress yet from an invalid operation that should never reach I/O. Molten expresses that distinction through explicit limits, credit accounting, transition decisions, and shell-owned payload queues. This article assumes the [fabric transport runtime contract](../../fabric-transport-session-runtime.md). Its examples explain inspected accounting rules, not recommended capacity settings or measured throughput.

## Bounds belong to different resources

`TransportLimits` includes listener and session counts, streams per session, frame and datagram sizes, queued-event and queued-byte limits, in-flight bytes, and an operation deadline window. The [contract types](../../../crates/molten-core/src/fabric_transport/mod.rs) keep these quantities separate because they constrain different resources. A legal frame can still exceed available credit; a session with spare credit can still be unable to admit another stream; an otherwise small operation can arrive after its deadline.

For a stream frame, payload validation uses the smaller of the transport profile's frame bound and the registered protocol's framing bound. Empty payloads and malformed payload references deny. Deadline admission checks require the deadline to be after the observed tick and within the profile window; frame submission rejects observations at or after the session deadline. These checks consume explicit ticks. They do not read a clock inside molten-core.

The [send transition](../../../crates/molten-core/src/fabric_transport/transition.rs) then computes prospective stream and session in-flight totals with checked arithmetic. It compares payload size with stream send credit and both prospective totals with the in-flight bound. Exhaustion returns a `Backpressured` event carrying `NotAttempted` delivery and a higher-level-policy retry disposition, with the transport state preserved. This is neither a successful submission nor evidence that a peer refused the payload.

## Progress is an accounting transition

On accepted submission, the stream loses payload-sized credit, its in-flight bytes grow, the session total grows, and the send sequence advances. The emitted `FrameSubmitted` event is `Pending`; its retry disposition warns that reconciliation is needed. Submission counters describe acceptance into this model, not application commits.

`AcknowledgeFrame` rejects zero bytes or an acknowledgement larger than either outstanding stream or session bytes. Accepted acknowledgement subtracts those bytes and replenishes stream credit, subject to the credit bound. `GrantCredit` is a separate explicit operation with checked addition and the same upper limit. Neither elapsed wall time nor repeated calls to the original send manufacture credit.

These details matter when composing limits. An acknowledgement both releases in-flight pressure and restores credit; a credit grant only changes credit. Consequently, granting more credit cannot authorize a submission that would exceed the session's aggregate in-flight limit. That separation prevents one stream's local credit from ignoring work already outstanding on other streams.

## Worked reasoning: credit is not the only bottleneck

Take an illustrative profile with a 4,096-byte in-flight limit and a 1,024-byte frame limit. Assume one stream has 512 bytes of credit and no in-flight data. A legal 768-byte frame is backpressured because it exceeds credit. No sequence number or in-flight counter is consumed by that decision.

Grant 256 additional bytes. If the session still has enough aggregate capacity, the same-sized submission now consumes all 768 bytes of credit and records 768 bytes in flight. An acknowledgement for 768 bytes restores the credit and returns those in-flight counters to their previous values. An acknowledgement for 769 bytes denies because it exceeds this outstanding amount.

Now change the starting state: other streams already account for 3,584 bytes in flight. Even with 768 bytes of credit available, the proposed submission would bring the session to 4,352 bytes and is backpressured. More credit does not solve this case; explicit completion or another terminal accounting path must change the outstanding work. The example illustrates arithmetic, not a scheduler fairness guarantee.

## Queue storage and bounded allocation are shell concerns

The pure helper `ensure_event_capacity` checks supplied queue counters. It checks a prospective extra event slot and the current queued-byte count; it is not a payload storage allocator or a complete model of every shell queue. Avoid reading a profile field as proof of a global process-memory bound.

The [cross-process effect port](../../../src/fabric_transport/cross_process/effect/parts/port/p000/body.rs) owns actual queued payload vectors. Registration checks payload count and checked aggregate payload bytes against the profile before adding the payload. Routing removes the payload and subtracts its accounted size. Those shell checks complement, rather than duplicate exactly, the pure stream accounting.

The [cross-process frame reader](../../../src/fabric_transport/cross_process/iroh/parts/shell/p002/body.rs) reads a fixed-size prefix, decodes the length, rejects zero or oversized frames, converts the length to the host index type, and only then allocates the payload buffer. It also checks stream termination after the exact payload. This supports a concrete bounded-allocation claim for that function, not a claim that every Iroh-internal allocation is covered by Molten's payload budget.

## Verification and limits

Suggested review should test exact bounds as well as one byte above them, distinguish invalid acknowledgement from backpressure, and compare state before and after a blocked send. The existing [flow-control test](../../../crates/molten-core/src/fabric_transport/tests.rs) asserts preserved state under backpressure and progress through explicit credit and acknowledgement. No test or performance benchmark was run for this article.

Finite bounds do not prove fairness, freedom from deadlock, application admission, or sufficient capacity for a workload. Queue accounting, protocol framing, kernel buffers, and canonical receipts remain different layers. The [ALPN registry contract](../../iroh-alpn-routing-registry.md) also requires resource evidence for router admission; negotiating the correct ALPN does not exempt a session from its transport limits.

## Sources

- [Technical companion](../README.md)
- [Fabric transport session runtime](../../fabric-transport-session-runtime.md)
- [Iroh ALPN routing registry](../../iroh-alpn-routing-registry.md)
- [Transport limits and state types](../../../crates/molten-core/src/fabric_transport/mod.rs)
- [Flow-control and deadline transitions](../../../crates/molten-core/src/fabric_transport/transition.rs)
- [Registered payload queue accounting](../../../src/fabric_transport/cross_process/effect/parts/port/p000/body.rs)
- [Bounded cross-process frame I/O](../../../src/fabric_transport/cross_process/iroh/parts/shell/p002/body.rs)
- [Transport behavioral tests](../../../crates/molten-core/src/fabric_transport/tests.rs)
