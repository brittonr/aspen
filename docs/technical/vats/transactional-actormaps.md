# Transactional actormaps

A vat makes a local group of object-capability behaviors a single transactional reasoning territory. This article explains the actormap boundary and the narrower reference-set laws exercised by Molten's fixtures. It assumes familiarity with actor turns and canonical content references. The [architecture](../../architecture.md#vatobject-layer-goblins-inspired) remains authoritative; this is part of the [Technical companion](../README.md), not a new transaction specification.

## A turn is the publication boundary

Molten's public local runtime remains actors, entities, facets, assertions, retractions, observations, and turns. A vat is optional internal structure hosted by an actor or service. Its actormap maps object references to behavior and state. Near calls can compose synchronously inside that territory without making intermediate changes externally committed.

The architectural sequence is: begin from the committed actormap, accumulate changes in a transactional view, retain outbound sends and dataspace actions as pending, then either publish the delta and pending actions or discard both. The important property is joint publication. Restoring object state while leaving a newly emitted capability reachable would not restore the pre-turn authority boundary. Conversely, discarding a message while retaining an object created only to serve that message can leave an unexplained object lifetime.

This is local transactional reasoning, not a distributed transaction protocol. A far target is outside the synchronous call chain. The architecture's rollback rule does not imply that an already executed remote effect can be reversed. Keeping effects behind admission and publication boundaries is therefore essential to the model, rather than an optimization added after transaction handling.

## The implemented finite law

The inspected implementation exposes `evaluate_actormap_transaction`, which validates an explicitly supplied `RuntimeActormapTransactionState` and constructs a predicate receipt. It does not execute arbitrary object behaviors. Its [validator](../../../src/runtime/predicates/parts/mod/p010/body.rs) checks canonical, sorted reference collections before applying set relationships.

Let `B` be `before_object_refs`, `A` the after set, `S` the spawned set, and `R` the removed set. For a committed transaction, the implemented membership equation is:

`A = (B minus R) union S`.

Spawns cannot already belong to `B`, and removals must belong to `B`. Every spawned object must be present and visible after commit. Removed objects must be absent from the after set, invisible, and absent from the reported used-object set. These checks connect membership with observability: matching only the membership equation is insufficient if a removed reference remains usable.

For rollback, the validator requires `A = B`. Spawned references cannot appear in the visible or used sets. The attempted spawn list can remain in the evidence describing the aborted transition; that does not make those objects live. This distinction lets an auditor explain an attempted change without confusing its description with its publication.

The related [rollback-cleanup validator](../../../src/runtime/predicates/parts/mod/p005/body.rs) checks equality of before and final snapshot references and rejects intersections between rolled-back references and remaining assertions, observers, pending calls, or authority snapshots. These are checks over supplied collections. They do not discover a hidden dependency graph or scan an external runtime for leaked references.

## Worked example: replacing a session helper

Consider an **illustrative** actor with root object `root` and helper `old`. During one turn it constructs `new`, removes `old`, and prepares an announcement advertising `new`. The names below stand for valid canonical references, not literal accepted reference strings.

For successful commit:

- before: `{root, old}`;
- spawned: `{new}`; removed: `{old}`;
- after and visible: `{root, new}`;
- used after removal: `{root}`.

The membership equation holds, the new object is published, and the removed reference is not used. If `old` remains visible, the validator denies the description even if the after set is correct.

Now suppose admission fails after `new` is constructed. A valid rollback description restores `{root, old}` and does not expose or use `new`. Keeping `new` in a pending announcement is an authority leak in the architectural model. The membership predicate alone cannot inspect that announcement; the caller must supply the relevant cleanup representation and keep actual pending actions aligned with the transaction result. This demonstrates why multiple local checks are useful without mistaking them for an end-to-end proof.

## Reviewing evidence and behavior

The [predicate evaluator](../../../src/runtime/predicates/parts/mod/p003/body.rs) hashes the transaction representation and binds it into the receipt input. Its check labels describe canonical references, delta commit, rollback preservation, spawn visibility, and removal invalidation. A canonical receipt preserves what was checked; it is not authorization to publish the described state.

The existing [vat property tests](../../../src/runtime/vat/parts/mod/tests/m000/p001/body.rs) generate spawn counts and cover successful commit, preserved rollback, and leaked-spawn denial. Suggested review extends beyond a happy-path receipt: compare object membership, reference visibility, and pending-action cleanup separately. The architecture documents `molten test vat run-fixture --out target/vat.preserves` and `molten test vat show target/vat.preserves` as fixture entry points. Those commands were not executed for this article.

## Limits and adjacent durability

The reference-set predicate does not inspect arbitrary behavior-state mutations, prove serializability across vats, establish storage durability, or provide crash recovery. The [addressable actor survival matrix](../../addressable-actor-runtime.md#survival-matrix) explicitly marks in-flight deltas unsupported. A committed in-memory turn and a durable semantic-event commit are different boundaries. Neither a fixture pass nor an actormap snapshot establishes production readiness or exactly-once external effects.

## Sources

- [Technical companion](../README.md)
- [Architecture: vat/object layer](../../architecture.md#vatobject-layer-goblins-inspired)
- [Addressable actor runtime](../../addressable-actor-runtime.md)
- [Actormap membership and rollback laws](../../../src/runtime/predicates/parts/mod/p010/body.rs)
- [Rollback cleanup checks](../../../src/runtime/predicates/parts/mod/p005/body.rs)
- [Predicate receipt construction](../../../src/runtime/predicates/parts/mod/p003/body.rs)
- [Vat property tests](../../../src/runtime/vat/parts/mod/tests/m000/p001/body.rs)
