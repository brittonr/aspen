## Context

Trellis is the right home for small, reusable, bounded predicates that Molten uses to decide whether a transition, admission decision, or persisted artifact is valid. It is not the right home for all runtime implementation details.

Molten runtime execution remains in Rust and adapters:

- actor scheduling,
- dataspace indexes,
- Preserves encoding and decoding,
- Iroh, Redb, Wasmtime, Steel, and CLI adapters,
- trace output,
- persistence effects.

Trellis should answer bounded questions such as:

- is this assertion visible given owners and retractions?
- is this turn allowed to commit?
- did this promise pipeline preserve order or fail causally?
- does this snapshot claim only authority already held?

## Goals

- Add Trellis-backed predicates for security-relevant local runtime invariants.
- Keep predicates finite and bounded so they are suitable for verification and property testing.
- Use predicates at admission/commit boundaries, not as a general-purpose runtime engine.
- Align predicate names and evidence with Cairn requirements and Molten trace/receipt records.
- Start with invariants that are likely to cause security or consistency bugs if implemented only informally.

## Non-Goals

- Do not port the whole actor runtime or dataspace index into Trellis.
- Do not encode arbitrary Preserves values in Trellis before a bounded pattern/value subset exists.
- Do not block the first local runtime prototype on all Trellis predicates being complete.
- Do not use Trellis predicates to justify side effects after the fact; admission happens before commit or adapter effects.

## Predicate Areas

### Dataspace assertion visibility

Predicate inputs should model:

- canonical assertion id,
- owner ids,
- owner live/dead/revoked state,
- per-owner assertion handles,
- current retraction set.

The core property:

```text
assertion is visible iff at least one live admitted owner maintains it
```

Duplicate assertions should be deduplicated for observers while retaining enough ownership information to retract only when the final live owner withdraws.

### Observe delivery and retraction propagation

Predicate inputs should model:

- current assertion set,
- current Observe subscription set,
- bounded Preserves pattern match result,
- prior delivered set,
- new assertion/retraction event.

Properties:

- new Observe receives matching current assertions,
- future matching assertions are delivered,
- matching retractions are propagated,
- Observe retraction stops future delivery and retracts forwarded values scoped to that subscription.

### Deterministic Preserves pattern matching

Molten needs a bounded Trellis-friendly subset of Preserves patterns. First predicates should cover:

- discard/wildcard,
- literals,
- records/arrays/dictionaries with deterministic key traversal,
- binding order.

The property is not full parsing; it is deterministic match/admission over a bounded model of values and patterns.

### Turn commit and rollback

Predicate inputs should model:

- prior actor/vat/dataspace state summary,
- pending actions,
- admission decisions,
- turn outcome: success, failure, denied.

Properties:

- pending actions are not visible before commit,
- successful admitted turns apply pending actions atomically,
- failed or denied turns leave committed state unchanged,
- trace/evidence records can describe failed attempts without making failed actions visible.

### Actormap transactions

Predicate inputs should model:

- prior actormap keys/state hashes,
- spawned/removed objects,
- object state deltas,
- turn outcome.

Properties:

- commit applies the delta atomically,
- rollback restores prior committed state,
- removed objects cannot be called as near references after commit,
- spawned objects do not become visible if the turn aborts.

### Near/far reference admission

Predicate inputs should model:

- caller vat id,
- target descriptor,
- reference scope,
- requested call kind: synchronous or asynchronous.

Properties:

- synchronous near call is admitted only when caller and target share the same vat and live turn,
- far reference calls are asynchronous only,
- session/persistence/sandbox boundaries force far semantics.

### Promise/vow state machines and pipelining

Predicate inputs should model:

- promise state: pending, resolved, broken, cancelled, timed out,
- queued pipelined calls,
- configured bounds,
- resolution/failure event.

Properties:

- pending can transition to exactly one terminal state,
- queued calls preserve order when forwarded,
- broken/cancelled/timed-out promises fail queued calls causally,
- configured queue/lifetime/payload bounds are enforced.

### Revocation cleanup

Predicate inputs should model:

- proxy/ref id,
- revocation state,
- dependent assertions,
- dependent Observe subscriptions,
- pending far calls,
- child references.

Properties:

- revoked references deny future use,
- dependent assertions/subscriptions are retracted,
- pending calls are cancelled or failed according to policy,
- child references do not remain live unless independently admitted.

### Safe serialization authority check

Predicate inputs should model:

- held authority set before snapshot,
- claimed authority set in snapshot portrait,
- admitted restore grants,
- upgrade recipe ids.

Properties:

- snapshot claims are a subset of authority already held or explicitly admitted,
- restore cannot mint new authority by object self-description,
- upgrade recipes cannot silently broaden authority.

### Service dependency admission

Predicate inputs should model:

- service demand assertions,
- dependency assertions,
- service states,
- reverse dependencies,
- restart/shutdown requests.

Properties:

- a service starts only when required dependencies are ready or explicitly force-run,
- removing demand eventually permits shutdown when reverse dependencies allow,
- failed dependencies prevent dependent readiness.

## Recommended Implementation Order

1. Dataspace assertion lifetime and deduplication.
2. Turn commit/rollback.
3. Deterministic Preserves pattern matching subset.
4. Observe delivery/retraction propagation.
5. Promise/vow state machine and bounded pipelining.
6. Revocation cleanup.
7. Actormap transaction predicates.
8. Near/far reference admission.
9. Safe serialization authority subset.
10. Service dependency admission.

## Open Questions

- Should these predicates live upstream in Trellis or in a Molten-local crate using Trellis style first?
- What bounded Preserves value model is sufficient for early pattern predicates?
- How should predicate evidence be named in Cairn receipts?
- Which predicates need Verus proofs immediately versus Rust model/property tests first?
