# Design: Coalesce equal-assertion notifications

## Context

`stage_step` computes observer notifications from committed state for the `Assert`, `Retract`, and `Observe` steps.
Assertion identity is the pair `(actor, value)`, so the assertion set treats one value asserted by two actors as two
entries. `cleanup_actor_scope` removes only the named actor's entries.

## Approach

Keep per-owner records. Decide visibility in a pure function over the committed assertion set, before the turn stages
notifications:

- `Assert`: emit `AssertionObserved` only when no other committed assertion carries an equal canonical value.
- `Retract`: emit `AssertionRetractionObserved` only when no other committed assertion carries that value after the
  retraction.
- `Observe`: emit one existing-match notification per distinct matching value.

The decision reads only committed state, so the staged-turn boundary, predicate receipts, and rollback behavior stay
unchanged. `ObserveRegistered` stays per subscription: two subscribers keep two subscriptions, and a subscription is
not coalesced.

## Decisions

### Decision: Keep per-owner records and coalesce at notification time

**Choice:** Keep the ordered set of `(actor, value)` assertions and compute visibility in the notification path.

**Rationale:** The stored per-owner record answers "who maintains this fact" and keeps per-owner retraction exact. A
bag refactor would change snapshot shape and canonical refs for no behavioral gain. Coalescing at notification time
also keeps the change inside one file plus tests.

## Risks / Trade-offs

- An observer that needs owner identity loses it, because one notification can have several owners. Owner identity
  stays visible in the assertion record and in per-owner retraction evidence, so the decision remains auditable.
- The reference harness keeps bag counting. Parity comparison must compare visible notifications rather than internal
  counts.
- Canonical value equality is the comparison basis. Runtime values are canonicalized on construction, so two spellings
  of one value cannot diverge.
