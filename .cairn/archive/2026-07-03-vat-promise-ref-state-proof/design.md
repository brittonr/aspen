# Design: vat promise and ref state proof

## Scope

This change proves vat promise, actormap transaction, near/far ref, distributed ref lifetime, revocation cleanup, and authority snapshot state machines. It covers pending/resolved/broken/cancelled promise states, bounded pipelining, transaction commit/rollback, handoff, stale-use denial, and cleanup of live refs.

## Proof checklist

- **Proof claim**: promise/ref transitions preserve locality and authority boundaries; rollback does not leak assertions/messages/effects; revocation cleanup removes dependent live refs; snapshots cannot mint authority not present in the source state.
- **Out of scope**: distributed scheduler fairness and transport delivery liveness.
- **Trusted assumptions**: runtime turn commit/rollback proof validates base dataspace mutation semantics.
- **Positive evidence**: valid pending→resolved and pending→broken traces, legal near/far routing, admitted handoff, committed actormap transaction, and revocation cleanup traces.
- **Negative evidence**: synchronous far call, stale distributed ref use, unresolved pipeline misuse, rollback leak, missing revocation cleanup, and authority amplification deny.
- **Canonical refs**: promise refs, pipeline refs, object refs, near/far refs, handoff refs, actormap refs, revocation refs, snapshot refs, and predicate receipt refs.
- **Regeneration command**: `cargo test vat runtime`.

## Functional core

Model promise/ref/actormap transitions as pure predicates over explicit state records and candidate operations. Runtime adapters emit receipts and effects only after predicate pass.

## Non-goals

- No new distributed object transport guarantee.
- No authority creation from serialization, snapshot, or handoff evidence alone.
