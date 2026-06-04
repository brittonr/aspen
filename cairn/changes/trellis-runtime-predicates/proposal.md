## Why

Molten's runtime architecture now includes SAM-style dataspaces, Goblins-style optional vat/object internals, Trellis choreographies, and Trellis Raft consensus. Trellis already covers choreography and many consensus/capability primitives, but the new local runtime semantics introduce additional safety-critical admission points: assertion visibility, Observe delivery, turn commit/rollback, actormap transactions, near/far reference rules, promise pipelines, revocation cleanup, safe snapshot authority, and service dependency startup.

These should not all be implemented only as ad hoc runtime code. Molten should add small, bounded Trellis predicates for the runtime invariants that matter at admission and commit boundaries, while keeping scheduling, indexing, adapters, encoding, persistence, and tracing in Molten Rust.

## What Changes

- Define a follow-up Trellis predicate roadmap for Molten runtime invariants.
- Add Trellis-backed predicates for dataspace assertion ownership, deduplication, visibility, and automatic retraction.
- Add predicates for Observe subscription delivery and retraction propagation over deterministic Preserves pattern matches.
- Add predicates for actor turn commit/rollback and pending-action visibility.
- Add predicates for vat actormap transaction commit/rollback.
- Add predicates for near/far reference admission so synchronous calls remain local to a vat and far calls remain asynchronous.
- Add predicates for promise/vow state machines and bounded promise pipelining.
- Add predicates for revocation cleanup of proxies, assertions, subscriptions, pending calls, and live references.
- Add predicates for safe serialization authority checks so snapshots cannot mint authority not already held.
- Add predicates for service dependency startup/shutdown admission.
- Keep Trellis as the bounded admission/spec layer; keep runtime execution and adapter effects in Molten.

## Impact

This change does not require all predicates before the runtime prototype exists. It records which invariants should graduate from tests and Rust validation into Trellis-backed logic as the runtime matures. The first candidates are assertion lifetime/deduplication, turn commit/rollback, deterministic pattern matching, promise state transitions, and revocation cleanup.
