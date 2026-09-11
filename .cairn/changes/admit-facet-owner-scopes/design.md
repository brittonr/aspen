# Design: Admit facet owner scopes

## Context

`RuntimeState` owns assertions, observers, and messages in ordered sets keyed by actor strings, and
`cleanup_actor_scope` removes entries for one actor. `docs/architecture.md` describes facets as part of the runtime
model, and the vat and system-extension layers already need nested lifetimes. Nothing in the source represents the
parent relation, the stop order, or the crash distinction.

## Approach

Add a facet record to the runtime state: facet id, parent facet id or actor root, and a state of `running | stopped`.
Assertions, observers, and child facets carry an owner facet id alongside the existing actor id.

Stop is a staged action, so it commits or rolls back with its turn like every other runtime action:

- Walk children depth-first and stop each one before its parent.
- Retract the stopping facet's assertions and observers after its children stop.
- Run the stop handler as the last staged step for that facet; a handler that asserts during stop is denied because
  the facet is already stopped.
- Record `stopped` on the facet and keep the record as a tombstone so a later action can prove permanence.

Crash handling stays separate: owner-scope cleanup retracts the same state and skips the handler. The tombstone is not
required on the crash path.

## Decisions

### Decision: Represent facets as an owner tree over the existing state

**Choice:** Add a facet record and facet ids to existing owner fields rather than a second dataspace or a separate
facet store.

**Rationale:** Assertions keep one owner table, so `Observe`, routing, and cleanup read one structure. A separate
store would need duplicate routing rules and would double the places where stop order can be wrong.

### Decision: Stop handlers run after assertion retraction

**Choice:** The handler is the final staged step of the stop turn, after children stop and after this facet's
assertions retract.

**Rationale:** The manual orders it that way (`05-glossary.md → Facet`), and a handler that runs earlier could
re-assert a facet-owned fact that then survives the stop.

## Risks / Trade-offs

- Tombstones retain one small record per stopped facet. Bound retention by the owning actor lifetime.
- A stop handler is a new execution surface. It runs inside the existing turn boundary with the existing policy
  gates, so it grants no new authority.
- Actors that never create a facet must keep identical behavior. The actor-scope cleanup path stays the owner of
  record for flat scopes.
