# Observation and Routing

Molten's local dataspace exposes both observation of maintained assertions and routing of envelopes to local subscribers. They share canonical values and a bounded pattern representation, but they are not interchangeable delivery mechanisms. This article assumes the [architecture's actor and envelope model](../../architecture.md), then separates matching, state change, routing, admission, and reference-harness evidence. Return to the [Technical companion](../README.md).

## The inspected pattern language

`RuntimePattern` currently has two variants: `Exact(RuntimeValue)` and `Wildcard { binding }`. Exact matching uses runtime-value equality, which compares canonical Preserves bytes. Wildcard matching validates its binding name, matches the entire candidate, and returns a binding paired with the candidate's value reference. This is not a general destructuring language or an arbitrary predicate evaluator. The [pattern implementation](../../../src/runtime/predicates/parts/mod/p000/body.rs) explicitly rejects unsupported AST forms.

The exact AST carries both a value and a declared value reference. Parsing recomputes the reference and rejects mismatch, avoiding acceptance of an AST that names one value while carrying another. Wildcard binding names must be nonempty, within the implementation's byte bound, and use ASCII alphanumeric, underscore, or dash bytes. Those checks concern representation and determinism; they do not authorize access to the matched value.

`from_observe_value` preserves a shorthand: recognized exact or wildcard AST records are parsed as patterns; other values become exact-value patterns. A record that is not recognized as one of the supported AST forms is therefore not necessarily interpreted as executable query syntax. This is an important review distinction when an application accepts arbitrary Preserves records from users.

## Current facts and subsequent changes

For a local `Observe` step, staging records the observer and walks the current assertion set, creating `AssertionObserved` events for matching assertions. For a later `Assert`, staging walks the current observers and creates matching observation events. `Retract` similarly creates `AssertionRetractionObserved` events. These events become successful turn output through the commit path, not merely because matching ran. See [state staging](../../../src/runtime/dataspace/parts/state/p001/body.rs).

The three operations explain why observation differs from a mailbox subscription that only notices future sends. A new observer can receive already maintained facts. It also explains why an owner field matters: two owners can maintain the same value, and the notification records which owner contributed it. Nothing here warrants collapsing all equal-valued owner events into one notification without checking the consumer's intended semantics.

Error behavior also deserves precision. The private state matching helper converts a pattern parse/match error into `false`; the explicit initial-delivery evaluator returns errors from parsing and matching. `LocalAdapter::observe_pattern` validates and returns a `Result`. These surfaces should not be described as having identical diagnostic behavior merely because they share `RuntimePattern`.

## Envelope routing is subject routing

`LocalAdapter` stores an ordered map from actor identifiers to ordered subscription sets. `route_envelope` creates the envelope boundary and compares subscriptions against `envelope.subject`, not the body. Once one subscription matches for an actor, it pushes one `LocalDelivery` and breaks that actor's inner loop. Two overlapping subscriptions therefore produce at most one local delivery per actor for this routing call. The [adapter source](../../../src/runtime/dataspace/mod.rs) shows the actual matching and break.

A registered actor with no subscriptions receives no delivery through this method. A matching actor receives a boundary value, but that routing decision is not evidence that a handler executed, committed, persisted output, or passed every surrounding authority gate. Debug tracing records the match and delivery count; those observations are not canonical admission receipts.

## Worked reasoning: overlapping readiness subscriptions

Consider an illustrative envelope whose subject is `"service.ready"` and whose body is a detailed service status record. Actor `dashboard` subscribes both to the exact subject and a wildcard; actor `logger` subscribes only to the wildcard; actor `worker` is merely registered.

Routing examines the subject. `dashboard` gets one delivery despite two possible matches; `logger` gets one; `worker` gets none. Changing only the body does not alter this pattern decision. Changing the subject to `"service.failed"` removes the exact match but leaves both wildcard subscribers eligible.

Now compare an assertion observation: if two producers maintain the exact readiness value before `dashboard` registers an `Observe`, initial state staging creates owner-specific events for both assertions. Envelope deduplication by recipient does not imply the same multiplicity rule for assertion observations. They answer different questions: which actors receive this envelope, versus which maintained contributions match this observer?

## Reference evidence and verification

The [reference harness](../../../src/runtime/dataspace/parts/syndicate/p001/body.rs) authorizes the Molten request before applying steps, previews fanout, and denies steps that exceed the supplied budget. It then compares canonical event references across the adopted local scenarios. That is scoped parity and resource evidence, not evidence of interoperable networking or inherited authority; the [governing reference boundary](../../syndicate-reference-harness.md) says so explicitly.

Suggested review cases are late observer registration, future assertion, retraction, overlapping local subscriptions, unsupported wildcard binding, and exact AST reference mismatch. The [existing routing tests](../../../src/runtime/dataspace/parts/tests/p000/body.rs) cover current/future/retraction observation, envelope matching, and deterministic harness fanout throttling. These are recommended inspection and execution targets, not claimed execution results here.

## Limits and non-claims

No routing result establishes freshness, remote availability, exactly-once handling, or service readiness outside the represented state. The bounded pattern subset is not the full Preserves pattern ecosystem. This article also makes no wire compatibility, distributed ordering, durable subscription, or production-readiness claim.

## Sources

- [Architecture and envelope spine](../../architecture.md)
- [Syndicate reference harness boundary](../../syndicate-reference-harness.md)
- [Pattern parsing and matching](../../../src/runtime/predicates/parts/mod/p000/body.rs)
- [State observation staging](../../../src/runtime/dataspace/parts/state/p001/body.rs)
- [Local envelope adapter](../../../src/runtime/dataspace/mod.rs)
- [Reference admission and fanout](../../../src/runtime/dataspace/parts/syndicate/p001/body.rs)
- [Routing and observation tests](../../../src/runtime/dataspace/parts/tests/p000/body.rs)
