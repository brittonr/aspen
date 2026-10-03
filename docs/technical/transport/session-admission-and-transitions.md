# Session Admission and Transitions

Transport establishment and peer admission answer different questions. This article explains their separate state machines, the evidence each consumes, and the mistakes caused by treating a connected socket as an authorization decision. Familiarity with the [fabric transport contract](../../fabric-transport-session-runtime.md) and [peer session relation](../../peer-session-transition-relation.md) is assumed. These governing documents remain authoritative; this companion describes the inspected implementation rather than introducing another admission policy.

## Two records, two meanings of progress

The fabric transport core operates on a `TransportState`, with protocol registrations and generation-scoped session and stream handles. `ScopedTransportId` contains an opaque reference, a service identity, and a generation. A handle is therefore not just a lookup key: its service and generation participate in admission. `open_session` checks the registered ALPN, registration scope, active registration phase, duplicate handles, session capacity, peer evidence, and deadline before returning a new active session. See the [transport transition implementation](../../../crates/molten-core/src/fabric_transport/transition.rs).

The peer relation instead models discovery and bootstrap progress. Its reviewed path is `Discovered → Invited → Handshaking → Negotiated → Admitted → Connected`, with explicit events between states. The relation also admits expiry, revocation, quarantine, and selected recovery transitions. These states are not aliases for fabric session phases. In particular, obtaining a transport `SessionEstablished` event does not mechanically justify moving a peer record to `Connected`.

That separation is visible in the [peer guards](../../../src/fsm/parts/p002/body.rs): `Connect` requires an authority reference, and the specified reference must occur in the prior record's authority references. Topic equality and the reviewed transition tuple are independent checks. Correct authority evidence for a different topic does not repair a wrong-topic transition.

## Evidence presence is not authority creation

`PeerIdentityRefs` separates transport identity from membership, application principal, trust decision, capability authority, and bootstrap policy. Its `has_service_authority` predicate accepts the normal four-reference combination or an explicit bootstrap policy reference. This is an admission predicate over supplied evidence, not a membership service or a capability minting operation. The [contract types](../../../crates/molten-core/src/fabric_transport/mod.rs) deliberately keep these references distinct.

Likewise, the peer relation checks relationships among supplied fields. It does not independently fetch a revocation registry or consult an ambient clock. Expiry checks in the inspected guard function require a nonzero `at_tick`; this article does not interpret that as a complete freshness protocol. Recovery is also constrained by the transition table: recovery to `Invited` exists for `Expired` and `Quarantined`, not `Revoked`. Supplying recovery evidence alone does not add a missing transition.

There is a source-level qualification around bootstrap admission. The governing peer document says missing bootstrap admission denies. The inspected `Admit` guard calls `missing(required_bootstrap_ref, bootstrap_refs)`, and `missing` returns false when the required reference is absent. Consequently, this implementation establishes rejection of a *specified but unavailable* bootstrap reference, not universal rejection when the optional field is omitted. No broader bootstrap guarantee is inferred here; that difference remains unresolved by this documentation change.

## Worked reasoning: a reachable but unadmitted peer

Consider an illustrative extension whose generation is 12. A remote endpoint authenticates at the transport layer and speaks the registered ALPN. Its peer evidence contains only a transport identity reference. An attempted normal fabric session admission still lacks membership, principal, trust, and capability references. Connectivity does not fill those fields; the transport's normal admission path rejects that evidence combination.

Now suppose the required service evidence is present, and the fabric session opens. A separate peer record is already `Admitted` for topic `inventory`. A proposed `Connect` transition observes topic `inventory` but supplies no required authority reference. The peer guard denies it despite the successful transport session. Supplying an authority reference not contained in the record also denies. These are distinct failures with the same practical lesson: do not derive authorization from reachability.

Finally, replace the registration with generation 13 and retain an old generation-12 handle in a queued callback. The transport scope checks prevent that callback from being interpreted as work belonging to the new owner. Generation scoping is a stale-owner defense; it is not evidence that the new owner has acquired additional application privileges.

## Reading a denial correctly

The [decision construction](../../../src/fsm/parts/p001/body.rs) preserves the prior state enum on denial but replaces the record's diagnostics. It hashes both the prior record and the resulting record. Therefore, “deny preserves state” does not necessarily mean the entire serialized record, or its hash, is unchanged. Reviewers should compare the transition decision and state fields while accounting for diagnostic changes. A receipt binds an evaluated relation; its existence alone is not a passing result.

## Verification and limits

Suggested review checks are: wrong topic with otherwise valid evidence; a skipped transition; `Connect` without authority; recovery from each terminal category; and a stale-generation transport handle. Inspect the existing [transport tests](../../../crates/molten-core/src/fabric_transport/tests.rs) for the identity-without-authority and ownership-transfer cases. These are verification suggestions, not executions performed for this article.

Neither state machine proves application success, durable messaging, consensus, or production readiness. A passing peer transition does not replace independent capability, policy, resource, or operation-specific gates.

## Sources

- [Technical companion](../README.md)
- [Fabric transport session runtime](../../fabric-transport-session-runtime.md)
- [Peer session transition relation](../../peer-session-transition-relation.md)
- [Transport contract types](../../../crates/molten-core/src/fabric_transport/mod.rs)
- [Transport transition laws](../../../crates/molten-core/src/fabric_transport/transition.rs)
- [Peer decision construction](../../../src/fsm/parts/p001/body.rs)
- [Peer transition guards and receipt encoding](../../../src/fsm/parts/p002/body.rs)
- [Transport behavioral tests](../../../crates/molten-core/src/fabric_transport/tests.rs)
