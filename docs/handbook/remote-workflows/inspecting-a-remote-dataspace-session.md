# Inspecting a remote dataspace session

Mode: How-to

## Goal and prerequisites

Determine which receiving session owns a remote assertion, whether that owner remains open, and what evidence supports cleanup or denial. You need the delivery envelope and transport receipt, the actual `RemoteSessionRegistry` used by the caller, and runtime snapshots or captured admission results. If you have only a peer address or a successful transport receipt, you do not yet have enough evidence to answer those questions.

This is a source-guided inspection procedure, not an invented session-status CLI. The paths below were source-checked; no runtime verification was performed for this batch. Return to the [Handbook](../README.md). The [assertion lifetime companion](../../technical/dataspaces/assertion-lifetimes-and-cleanup.md) explains why equal payloads can have different owners.

## 1. Decide which application path actually ran

Locate the caller of `admit_and_apply_delivered_envelope_for_session`. Do not assume that every delivered envelope passes through it: the module also exposes non-session application helpers. A transport `Delivery` contains an envelope and receipt; session ownership appears only when a caller supplies a session reference to the session-aware path.

If your evidence came from `apply_delivered_envelope` rather than the session-aware function, stop interpreting the resulting actor identity as a receiving-session owner. Document the actual call path and ask the integration owner for its lifetime policy. The architecture requires cleanup of maintained facts; the existence of a cleanup helper alone does not prove the integration invokes it.

## 2. Match the receiving identity

Read the [session model](../../../src/remote/parts/dataspace/p006/body.rs). A session contains `session_ref`, `owner`, `receiver_peer`, `topic`, `generation`, and `state`. Its canonical reference binds receiver, topic, and generation. The readable owner string is derived separately as `session:{receiver_peer}/{topic}:{generation}`.

Compare all three identity inputs with the receiving context. An unchanged peer name does not mean an unchanged session after reconnect. Reopening a recorded identity is rejected, including after closure; a new generation produces a different owner and reference.

The inspected admission helper checks whether the declared session exists and is open. It does not itself compare the session's receiver and topic with the delivered envelope. Treat that as a caller-binding review obligation, not proof of an observed exploit or a reason to bypass admission.

## 3. Separate transport checks from admission evidence

For live delivery, [the transport helper](../../../src/remote/parts/dataspace/p001/body.rs) checks the subscribed topic, receiver or wildcard target, and locally available content bytes against their references. Neighbor changes and lag notifications return no delivery from this helper. A `NeighborDown` observation alone is not evidence that session cleanup ran.

Next inspect `DeliveryEvidence`: peer-bootstrap, capability, policy, resource, and authority reference lists. [Evidence validation](../../../src/remote/parts/dataspace/p003/body.rs) requires each category to be nonempty and syntactically valid. It also checks that envelope-declared capabilities and evidence references are represented in the appropriate supplied lists.

Those are concrete checks, not a complete verification of every referenced policy or authority artifact. Keep the caller's substantive admission decision and artifact resolution evidence alongside the receipt. Never manufacture reference-shaped strings to satisfy missing categories.

## 4. Inspect the admitted assertion

For an assertion, `SessionApplied` includes an applied-assertion value. Inspect its `assertion-ref`, `owner`, `session-ref`, `session-state`, `envelope-ref`, `operation`, and `payload-ref`. Compare the owner with the assertion actor in the runtime snapshot rather than searching only for the payload.

The checked-in [ownership fixture](../../../src/remote/parts/dataspace/tests/m000/p003/body.rs) provides a concrete example: peer `a`'s producer sends `<service-ready "db">` to `peer:b` on `services`; generation 1 is opened at the receiver. The stored assertion belongs to the receiving session, not merely to the remote producer string. The test inspects both the committed event and the retained assertion.

## 5. Decide whether closure completed

Follow [session closure](../../../src/remote/parts/dataspace/p007/body.rs). It gathers session-owned assertions, applies explicit retractions, cleans the actor scope, marks the registry entry closed, and constructs lifecycle cleanup evidence. Useful outputs include `retraction_events`, `cleanup`, `cleanup_decision`, `cleanup_receipt_ref`, and before/after state references.

Require two observations: no remaining assertion owned by the closed session, and the relevant observer retraction events when those observers existed. A changed state hash alone does not explain which facts disappeared. Equal payloads maintained by another owner may legitimately remain visible.

The fixture checks a consumer observing readiness, closes the session for disconnect, and expects both retraction and observer notification. This is source evidence for the scenario, not a claim that every live disconnect is wired to it.

## 6. Handle late delivery and replay safely

The late-delivery fixture expects a denial for a closed owner and unchanged runtime state. Session-aware replay for an unknown or closed owner returns diagnostic-only results and requires peer reassertion; it does not resurrect old facts. A non-replayable log is rejected.

Stop if the only proposed recovery is deleting the registry, reusing the old identity, or replaying historical readiness as current authority. Preserve the old evidence, obtain a newly admitted session through the owning integration, and require fresh assertions. Report the inspected call path, owner, generation, closure evidence, and any missing binding or lifecycle wiring explicitly.

## Sources

- [Handbook](../README.md)
- [Architecture and maintained assertions](../../architecture.md)
- [Assertion lifetimes and cleanup companion](../../technical/dataspaces/assertion-lifetimes-and-cleanup.md)
- [Session identity and admission](../../../src/remote/parts/dataspace/p006/body.rs)
- [Closure and session-aware replay](../../../src/remote/parts/dataspace/p007/body.rs)
- [Session ownership, late delivery, and reconnect fixtures](../../../src/remote/parts/dataspace/tests/m000/p003/body.rs)
- [Delivery evidence checks](../../../src/remote/parts/dataspace/p003/body.rs)
