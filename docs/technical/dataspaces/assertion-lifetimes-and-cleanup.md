# Assertion Lifetimes and Cleanup

An assertion is maintained state with an owner, not an unqualified historical event. This article follows ownership through the local snapshot, visibility evaluation, explicit retraction, and actor-scope cleanup. Read the [architecture's dataspace model](../../architecture.md) first; its lifecycle language includes actor, session, facet, and live-reference ownership, while the inspected local state API specifically exposes actor-scoped cleanup. Return to the [Technical companion](../README.md).

## Identity has two levels

`RuntimeAssertion` contains an actor string and a `RuntimeValue`. The value retains canonical Preserves bytes and a content reference; its equality compares canonical bytes. The assertion's canonical record includes the actor as well as the value and value reference. Thus two actors maintaining identical content contribute two distinct owned assertions, while an identical actor/value pair occupies one entry in the ordered assertion set. These mechanics appear in the [runtime value types](../../../src/runtime/turn/mod.rs) and [snapshot model](../../../src/runtime/dataspace/mod.rs).

That distinction prevents one owner's withdrawal from erasing another owner's contribution. The relevant unit for retraction is the actor/value pair, not only the value reference. Conversely, the same owner repeating the same assertion does not create a stack of independently retractable leases in this model. Applications requiring distinct lifetimes cannot infer them from repeated insertion of an identical pair.

Visibility is a separate computation. `evaluate_assertion_visibility` receives a snapshot, a value, and an explicit set of live owner identifiers. It collects matching assertion references only when their actors appear in that set, sorts those references, and declares visibility when the collection is nonempty. It does not discover liveness through a clock, socket, process table, or remote failure detector. The [visibility predicate](../../../src/runtime/predicates/parts/mod/p002/body.rs) makes that input dependency explicit.

## Explicit retraction versus scope cleanup

A `Retract` step stages an owner-specific removal and an `AssertionRetracted` event. It also stages `AssertionRetractionObserved` for observers whose patterns match the value. Successful commit applies the set removal. In the inspected staging code these events are built without first testing whether the owned assertion exists, so an event trace must not be interpreted as a count of actual set entries removed.

`cleanup_actor_scope` has a different shape. It gathers canonical references for assertions and observers owned by the named actor, and messages whose sender **or recipient** is that actor. It sorts those references, removes the corresponding entries, and returns a `RuntimeScopeCleanup` with `actor`, `assertion_refs`, `observer_refs`, and `message_refs`. The [state implementation](../../../src/runtime/dataspace/parts/state/p001/body.rs) performs all fallible reference construction before its retain operations.

This cleanup method does not itself construct a vector of normal retraction-observation events. It is therefore important not to conflate an ownership cleanup summary with observer notification delivery. The architectural requirement for automatic cleanup needs lifecycle integration that detects the relevant end of ownership and invokes the appropriate mechanism; an exposed cleanup method alone does not prove every disconnect or authority loss is wired to it.

## Worked reasoning: two readiness owners

Consider an illustrative local state in which `worker-a` and `worker-b` both maintain `"service.ready"`, and `dashboard` observes that value. Suppose there are also messages `dashboard → worker-a` and `worker-b → audit`.

Before failure, visibility evaluation with both workers in the live-owner set returns visible with two owned assertion references. If `worker-a` is removed from that input set, visibility can remain true through `worker-b` even before physical cleanup removes the stale entry. This is a logical visibility filter, not a claim that stale storage has already disappeared.

Next invoke actor-scope cleanup for `worker-a`. Its assertion and any observer registered under that actor are removed. The message addressed to `worker-a` is also removed even though another actor sent it. The unrelated `worker-b → audit` message and `worker-b` assertion remain. Visibility is still true for the shared readiness value. Only removal or exclusion of the final live owner's assertion makes that value invisible to the visibility predicate.

A dashboard that treats every owner-specific retraction as global loss of readiness would therefore be reasoning at the wrong identity level. Its desired aggregate interpretation must be grounded in the live owned-assertion set, not an assumed one-to-one correspondence between notifications and globally present values.

## Verification and review guidance

Suggested review cases include two owners with equal values, duplicate insertion by one owner, retraction by the wrong owner, and cleanup of messages in both directions. Inspect both the returned reference lists and the retained snapshot; a plausible summary alone is insufficient. Separately review how the caller supplies live owners and how lifecycle cleanup reaches observers.

The [predicate tests](../../../src/runtime/predicates/parts/mod/tests/m000/p000/body.rs) include duplicate ownership until final retraction. The [dataspace tests](../../../src/runtime/dataspace/parts/tests/p000/body.rs) include reference-harness ownership cleanup. They are source references for review, not a report of tests executed during this documentation change.

## Limits and non-claims

The local actor string is an ownership key, not independent capability proof. Canonical cleanup references identify what the model removed; they do not establish remote deletion, durable erasure, retention permission, or distributed garbage collection. The [Syndicate boundary](../../syndicate-reference-harness.md) likewise treats reference observations as diagnostic/parity evidence rather than imported authority. The architecture's broader lifetime model is not reduced to this one local API, and this article does not claim complete lifecycle automation from it.

## Sources

- [Architecture and assertion lifetimes](../../architecture.md)
- [Syndicate reference boundary](../../syndicate-reference-harness.md)
- [Canonical assertion and value types](../../../src/runtime/turn/mod.rs)
- [Actor-scope cleanup and retraction staging](../../../src/runtime/dataspace/parts/state/p001/body.rs)
- [Live-owner visibility predicate](../../../src/runtime/predicates/parts/mod/p002/body.rs)
- [Ownership visibility tests](../../../src/runtime/predicates/parts/mod/tests/m000/p000/body.rs)
- [Reference-harness cleanup tests](../../../src/runtime/dataspace/parts/tests/p000/body.rs)
