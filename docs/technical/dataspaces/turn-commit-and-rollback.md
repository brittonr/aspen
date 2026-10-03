# Turn Commit and Rollback

This article explains the local dataspace turn boundary: how pending actions become a committed snapshot, what rollback preserves, and what a transition receipt actually establishes. It assumes the actor/assertion vocabulary in the [architecture](../../architecture.md). It describes inspected in-memory mechanisms, not a distributed transaction protocol. Return to the [Technical companion](../README.md).

## Pending state is not committed state

The architecture describes an actor computing pending assertions, messages, and effects, passing admission, and then committing or rolling back. The concrete local representation separates `PendingTurn.actions` from `PendingTurn.events`. `RuntimeState::begin_turn` takes an immutable reference to the state and stages a step; it does not install the staged action. This distinction matters because events named `AssertionCommitted` or `MessageDelivered` are constructed during staging. Their names describe the intended committed interpretation, not permission to publish them before the enclosing operation succeeds. See the [state implementation](../../../src/runtime/dataspace/parts/state/p001/body.rs).

The committed snapshot contains logical time, random-generator state, effect sequence, messages, assertions, and observers. Messages, assertions, and observers are ordered sets. `committed_turn_snapshot` clones the supplied before-state and applies action-specific set operations: insert a message, observer, or assertion; remove an assertion on retraction. It does not infer authority from the operation's presence. The [transition law](../../../src/runtime/predicates/parts/mod/p006/body.rs) is therefore an explicit state transformation rather than a hidden action interpreter with ambient effects.

Set identity and event multiplicity are different questions. Repeating an identical assertion insertion need not grow the assertion set, but staging still constructs the associated events. Likewise, a message set is not evidence of exactly-once message processing. A consumer reviewing retries must examine event handling and effect admission separately from final set equality.

## Preview, validate, publish

`commit_turn_with_predicate_receipt` snapshots the current state, clones a preview, applies the pending turn to that preview, and evaluates the transition predicate over before-state, turn, after-state, and committed outcome. Only a passing predicate replaces the live state with the preview. An evaluation error or denied predicate leaves that replacement unperformed.

`evaluate_turn_transition` compares the provided after-state with `expected_turn_snapshot`. For committed outcomes the expectation is the action delta; for rolled-back, denied, and failed outcomes it is the unchanged before-state. Its receipt binds before and after snapshot references, the outcome, action summaries, and event references. This is useful evidence against a receipt being detached from the transition it describes; it is not a new authorization credential. The [predicate implementation](../../../src/runtime/predicates/parts/mod/p002/body.rs) shows the comparison and receipt construction.

Rollback itself is deliberately small. `rollback_turn` takes an immutable state reference, consumes the pending turn without applying it, and returns a `TurnRolledBack` event with actor and reason. The receipt-bearing rollback evaluates a denied outcome against the same before-state. This is discard-before-publication, not compensating work that attempts to undo an already observed external action.

## Worked reasoning: denied readiness

Consider an illustrative producer asserting the exact value `"service.ready"` while a consumer already observes that value.

1. Capture snapshot S before calling `begin_turn`.
2. Staging constructs the assertion action and the prospective assertion/observation events. S remains unchanged.
3. Suppose surrounding admission denies the producer. Rollback returns its denial event; neither the owned assertion nor the staged observer notification becomes committed through this operation.
4. A later admitted turn may assert the same value. Its preview contains the producer-owned assertion, and successful transition validation publishes that preview.

The crucial test is not merely that rollback returns a denial string. It is that every tracked component of the snapshot equals S afterward, and that staged success events were not released as successful output. The existing [rollback regression](../../../src/runtime/dataspace/parts/tests/p001/body.rs) checks unchanged staging and rollback snapshots, then contrasts a committed assertion.

## Effects and admission remain separate

The same state module handles `Clock` and `Random` through separate request/response paths. They are not ordinary `TurnAction` insertions; `begin_turn` does not stage them as dataspace actions. The local deterministic responses and recorded-response transition are not ambient clock or entropy access. Consequently, a claim that a dataspace rollback undoes arbitrary external I/O would exceed this implementation.

Similarly, structural transition validation does not replace policy, capability, or budget gates. The [reference harness contract](../../syndicate-reference-harness.md) preserves Molten admission boundaries even when local Syndicate observations agree. Transition correctness answers whether the declared state delta is right; admission answers whether that operation may occur.

## Verification and limits

Suggested review follows three boundaries: compare snapshots before and after staging; inspect the predicate's bound inputs; then inspect the caller that releases events or invokes adapters. Include duplicate assertions, absent retractions, denied turns, and stale after-snapshots rather than only successful insertion. These are proposed checks, not commands executed for this article.

This account establishes neither crash durability nor cross-process atomicity, scheduler fairness, external-effect rollback, or production readiness. Canonical references derive from Preserves values, not Rust memory layout. A passing local law remains scoped to its explicit inputs and represented state.

## Sources

- [Architecture and turn semantics](../../architecture.md)
- [Syndicate reference boundary](../../syndicate-reference-harness.md)
- [State staging, commit, and rollback](../../../src/runtime/dataspace/parts/state/p001/body.rs)
- [Snapshot transition law and bound inputs](../../../src/runtime/predicates/parts/mod/p006/body.rs)
- [Transition predicate](../../../src/runtime/predicates/parts/mod/p002/body.rs)
- [Rollback and commit regression tests](../../../src/runtime/dataspace/parts/tests/p001/body.rs)
