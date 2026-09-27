# Near and far reference semantics

Near and far classify a call boundary, not a network-distance estimate. This article separates local call admissibility, canonical object descriptors, and distributed reference lifetime. It assumes the actor-turn model in the [architecture](../../architecture.md#vatobject-layer-goblins-inspired). The [Technical companion](../README.md) provides navigation; the governing architecture and [addressable actor profile](../../addressable-actor-runtime.md) retain their respective authority.

## Locality is a semantic property

A near reference denotes an object in the same vat, where synchronous call/return is permitted within an actor turn. A far reference crosses the vat boundary and is asynchronous. Two vats sharing a process remain distinct territories. Conversely, the synchronous eligibility of a near reference says nothing about the latency or complexity of its behavior.

This split prevents a local-looking function call from implicitly waiting for a remote actor to finish. It also defines rollback scope: the near-call chain can be reasoned about within the enclosing transaction, whereas a far call is a pending outbound action with a later result. The architecture describes promises as the result of far calls. It does not make a remote response part of the caller's original synchronous transaction.

These are capability semantics. Knowing an actor name or endpoint is not an ambient route to the authority held by its objects. Routing, authority, and call mode are distinct questions even when their answers are recorded together.

## Descriptor identity versus call admission

The [vat descriptor implementation](../../../src/runtime/vat/parts/mod/p000/body.rs) defines `VatObjectRef` with `vat_id`, `object_id`, `kind`, and `authority_refs`. Its constructor sorts and deduplicates authority references. Its `value` method emits a `vat-object-ref-v1` record, and `object_ref` hashes that canonical value. Thus descriptor identity includes the represented authority list; it is not a Rust address or a hash of debug formatting.

That construction is not itself full admission. A content-addressed description can represent an invalid or unusable attempted call. The separate [near/far validator](../../../src/runtime/predicates/parts/mod/p005/body.rs) checks a canonical reference, nonempty caller and target vat identifiers, and `is_live`. It then examines reference kind and call mode:

| Supplied reference situation | Local predicate behavior |
|---|---|
| Live near reference, same vat, synchronous call | No locality or mode violation |
| Near reference, different vat | Denied, including for asynchronous mode |
| Far reference, synchronous call | Denied |
| Live far reference, asynchronous call | No synchronous-call violation |
| Non-live reference | Denied regardless of call mode |

Other input-shape checks still apply. In particular, “no locality violation” does not mean every other admission gate passed. The inspected validator does not require different vat identifiers for a far reference, and it does not require every near call to use synchronous mode. The architecture's positive description of synchronous near calls should not be strengthened into either unimplemented restriction.

`VatReferenceKind` also includes `Proxy`, while the near/far predicate matches only its own `Near` and `Far` classification. A proxy descriptor is therefore not evidence of a third call mode. Proxy mediation and target-call eligibility require separate reasoning.

## Lifetime is independent of descriptor stability

A canonical descriptor can retain identical bytes after its session becomes unusable. The [distributed lifetime validator](../../../src/runtime/predicates/parts/mod/p004/body.rs) makes this explicit through session liveness, pending and failed call sets, attempted uses, and optional admitted handoff.

Without a live session or admitted handoff, every reported pending call must appear in the failed set. An attempt to use the stale far descriptor is denied whenever the session is not live. An admitted handoff requires a replacement reference, and attempted uses must be confined to that replacement. A live session combined with either handoff admission or a replacement is rejected by this model.

These checks evaluate a supplied lifetime description. They do not establish how transport detects failure, when a partition ends, or who authorizes handoff. A replacement field does not automatically authorize migration.

## Worked example: local deployment, disconnected session

In an **illustrative** deployment, a caller vat and a catalog vat share a machine. The caller holds a live far catalog reference and initiates an asynchronous lookup. Co-location does not permit changing this into a synchronous far call: the mode restriction follows the reference boundary.

The session then disconnects with two pending calls, `lookup-a` and `lookup-b`. Without handoff, a lifetime description marking only `lookup-a` failed is incomplete and is denied. With an admitted replacement, a later attempt against the old descriptor is still denied; handoff is not a license to keep both reference paths active. A later replacement call needs its own applicable admission.

The [addressable actor wake rules](../../addressable-actor-runtime.md#wake-behavior) reinforce the boundary: a connection wake creates a new runtime connection and does not assert survival of a prior stream or session. Durable actor identity therefore cannot be used as evidence that a particular old far reference remains live.

## Verification and non-claims

Suggested review separates three inputs: descriptor identity, call-mode/liveness evidence, and distributed-session evidence. Reviewers can inspect `near_far_calls` in the [vat fixture](../../../src/runtime/vat/parts/mod/p001/body.rs) and the distributed cases in [the lifetime fixture](../../../src/runtime/vat/parts/mod/p002/body.rs). This article did not execute those fixtures.

Neither same-vat admission nor a lifetime receipt grants general invocation authority, authenticates a transport peer, guarantees message delivery, or proves remote effects absent after disconnect. These finite checks do not establish whole-system liveness, failure-detector accuracy, or production readiness. Their value is a precise local refusal boundary whose inputs can be reviewed independently of transport behavior.

## Sources

- [Technical companion](../README.md)
- [Architecture: vat/object layer](../../architecture.md#vatobject-layer-goblins-inspired)
- [Addressable actor runtime](../../addressable-actor-runtime.md)
- [Canonical object descriptors](../../../src/runtime/vat/parts/mod/p000/body.rs)
- [Near/far admission validator](../../../src/runtime/predicates/parts/mod/p005/body.rs)
- [Distributed reference lifetime validator](../../../src/runtime/predicates/parts/mod/p004/body.rs)
- [Near/far fixture construction](../../../src/runtime/vat/parts/mod/p001/body.rs)
- [Distributed reference fixture construction](../../../src/runtime/vat/parts/mod/p002/body.rs)
