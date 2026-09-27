# Revocable proxies and rights

Object-capability structure can narrow authority, make future use revocable, and enable private cooperation without turning identity into ambient permission. This article explains how Molten's architectural proxy model relates to its implemented revocation, object-authority, and rights-amplification predicates. It assumes familiarity with held references and actor turns. The [architecture](../../architecture.md#vatobject-layer-goblins-inspired) is authoritative; this article belongs to the [Technical companion](../README.md).

## Three operations on authority

Attenuation restricts what a holder can do through a reference. Revocation removes a previously available use path and requires associated cleanup. Rights amplification lets cooperating objects recover authority packaged for that cooperation. These operations are not interchangeable.

The architecture describes proxies that narrow authority, log use, transform or filter payloads, or cancel access. A proxy is useful because the caller can hold the mediator rather than the target's unrestricted reference. However, architectural support for this structure is not evidence that the inspected vat fixture implements a generic payload-filtering or logging proxy engine. Its `VatReferenceKind::Proxy` is a represented descriptor kind, and its local predicates validate specific authority relationships.

Revoking a proxy also does not erase every route to the underlying object. If another holder independently possesses a direct admitted reference, removing the proxy path says nothing about that holder's path. The scope of the revoked set is therefore central to interpreting cleanup evidence.

## Revocation as a checked cleanup boundary

The [revocation validator](../../../src/runtime/predicates/parts/mod/p005/body.rs) validates sorted canonical collections for revoked references, attempted uses, remaining assertions, remaining subscriptions, remaining pending calls, and remaining children. It then rejects intersections between the revoked set and each of the other sets.

This detects both explicit use after revocation and reported residue. Dropping a reference from a call table while keeping its dependent subscription represented as live is not complete cleanup under this model. Similarly, a queued call can retain a use path after the public-facing reference has disappeared.

The implementation compares the supplied reference sets; it does not discover dependency edges or traverse an arbitrary object graph. The [receipt evaluator](../../../src/runtime/predicates/parts/mod/p003/body.rs) labels checks for cleaned assertions, subscriptions, calls, and children, but these labels must be interpreted with the actual representation. A caller that omits a dependency from its input cannot turn the resulting pass into proof that the dependency never existed.

Revocation is about subsequent admitted use and cleanup. It is not an undo operation for an external effect already released. The [addressable actor profile](../../addressable-actor-runtime.md#unknown-effects) separately treats effects that may have occurred without terminal evidence as unknown, rather than resolving uncertainty by cancellation terminology.

## Rights amplification without authority creation

`RuntimeRightsAmplificationState` identifies the holder, sealed value, sealer and unsealer brands, and sealed and recovered authority collections. The [rights validator](../../../src/runtime/predicates/parts/mod/p004/body.rs) checks canonical references, nonempty sealed and recovered sets, matching brands, and containment of recovered authority within sealed authority.

A useful interpretation is recovery, not creation. If `S` is the sealed set and `R` the recovered set, the checked law is `R` contained in `S`, with both nonempty. Equality is not required: a cooperating object can recover a narrower subset. Matching brands alone cannot justify returning an unrelated extra authority reference.

The [rights fixture](../../../src/runtime/vat/parts/mod/p002/body.rs) constructs a `vat-sealed-value-v1` containing a brand reference and a sealed root-authority reference. It records a matching-brand recovery, a wrong-brand denial, and an over-recovery denial. The fixture's explicit Preserves representation is not evidence of encrypted secrecy or an unforgeable cryptographic container. The inspected predicate checks supplied references and set relationships; it does not decrypt bytes or independently establish possession of a private unsealer.

## Worked example: delegated catalog administration

Suppose an **illustrative** catalog owner gives an editor a proxy permitting metadata updates but not deletion. Separately, two internal objects cooperate through a branded sealed package holding one administrative reference.

When the editor's access is revoked, an adequate cleanup description removes use of the proxy and its represented pending-call and subscription dependencies. Merely hiding the proxy from a user interface while reporting a remaining revoked subscription fails the intersection check. A different administrator's direct reference is outside this particular revocation unless explicitly included; describing proxy revocation as “the catalog can no longer be changed” would overstate its scope.

For the cooperating objects, recovering the sealed administrative reference with the matching brand satisfies the rights relation. Returning that reference plus a storage authority absent from the sealed set is denied even if the brand matches. Using the correct sealed set with an unrelated brand is also denied. These independent failure cases demonstrate why neither caller identity nor an apparently plausible object name substitutes for the represented cooperation boundary.

## Endowment, admission, and review

The [object-authority validator](../../../src/runtime/predicates/parts/mod/p004/body.rs) requires the requested reference to belong to both endowed and admitted sets, and requires the entire endowment set to be contained in the admitted set. Possession and policy admission therefore remain separate inputs. The [ambient-authority fixture](../../../src/runtime/vat/parts/mod/p002/body.rs) records unendowed attempts across authority kinds and a specifically endowed/admitted clock case. It does not perform ambient clock, filesystem, or network effects in the pure law.

Suggested review uses `molten test vat rights-fixture --out target/vat-rights.preserves`, documented in the architecture, and inspects both expected denial cases as well as recovery. That command was not executed for this article. Review actual dependency enumeration separately from the revocation set calculation, and distinguish reference syntax validation from proof that evidence was admitted.

## Limits

These fixtures neither implement universal proxy interception nor establish cryptographic sealing, global revocation propagation, remote-effect reversal, or production isolation. A passing rights or cleanup receipt records a local checked relationship; it grants no new policy, transport, resource, or execution authority. Those boundaries remain necessary even when the object-capability graph is internally consistent.

## Sources

- [Technical companion](../README.md)
- [Architecture: vat/object layer](../../architecture.md#vatobject-layer-goblins-inspired)
- [Addressable actor unknown effects](../../addressable-actor-runtime.md#unknown-effects)
- [Vat reference descriptor kinds](../../../src/runtime/vat/parts/mod/p000/body.rs)
- [Revocation cleanup checks](../../../src/runtime/predicates/parts/mod/p005/body.rs)
- [Revocation receipt construction](../../../src/runtime/predicates/parts/mod/p003/body.rs)
- [Object-authority and rights checks](../../../src/runtime/predicates/parts/mod/p004/body.rs)
- [Rights and ambient-authority fixtures](../../../src/runtime/vat/parts/mod/p002/body.rs)
