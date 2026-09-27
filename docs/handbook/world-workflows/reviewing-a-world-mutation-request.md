# Reviewing a world mutation request

Mode: Review checklist

Use this checklist at the handoff between an operator requesting a mutation and the owner composing its execution adapters. The deliverable is a bounded review decision with referenced evidence, not an assertion that every preceding document is trustworthy. This checklist was source-reviewed, not exercised against a live deployment. The [Handbook](../README.md) links practical inspection paths; [preview-first composition](../../technical/world-effects/preview-first-operator-composition.md) supplies the theoretical background.

## Establish the review packet

- [ ] Does the packet name one immutable request, its exact reviewed preview identity, and the candidate or subject identities? Require actual artifacts, not screenshots of shortened references. Preserve the relationship between workflow and component plans.
- [ ] Is the proposed mutation classified correctly? Workflow checkpoint, branch, run, promote, and import have mutation argument structures in the [CLI](../../../src/cli/runtime/world.rs). A multi-operation graph belongs to `world plan`; a single-operation command cannot silently discard other operations.
- [ ] Is the request native JSON with the declared closed fields? Identify unknown fields, raw commands, missing expected head, duplicate operation identities, unresolved dependencies, and cycles as request problems, not reasons to weaken parsing.
- [ ] Is the claimed execution surface actually available? Record whether this is standalone planning, an explicit handler embedding, a fixture, or a live adapter. The standalone workflow apply path denies even with a matching preview reference.

Acceptance evidence is the retained request and canonical preview plus a named composition owner. If no live composition exists, approve only the planning review, not execution readiness.

## Verify identities, limits, and profile boundaries

- [ ] Do branch, expected head, generation, policy, authority observation, and profile references match across the request and review packet? A historical authority root is not the current observation required for mutation.
- [ ] Does every operation have an exact profile and supported state? Blocked, unsupported, and unavailable are meaningful outcomes. Reject any proposed substitution of local-head behavior for witnessed-head evidence or logical restoration for an opaque mismatch.
- [ ] Are resource limits explicit and appropriate for the graph and linked evidence? In the [logical fixture](../../../tests/fixtures/world-operator/logical/request.json), 13 operations fit its declared 32-operation bound. That is a fixture fact, not a recommended deployment limit.
- [ ] Are canonical Preserves and the correct BLAKE3 identity domains the basis for comparison? Do not accept Rust serialization layout, output-path names, or matching operation counts as identity evidence.

## Verify the actual mutation fence

- [ ] Does the embedding recompute and compare the submitted preview before execution? Inspect the [operator service](../../../src/world_operator/service.rs), not only a UI confirmation dialog.
- [ ] Is each handler registered once under the correct component owner, and does its preview return a planned outcome before execution?
- [ ] Which current-facts adapter supplies fresh head, generation, policy, authority, and profile facts immediately before each mutating operation? Require its observation source and lifetime, not just an `admitted` boolean in JSON.
- [ ] If a later operation blocks or becomes unknown, does the review preserve earlier effects rather than describe a whole-workflow rollback? Require the completed prefix and first blocker; later operations must not be claimed complete.

Acceptance evidence includes adapter bindings and the exact denied or admitted transition. Receipt links describe evidence; they do not grant mutation, dispatch, or deletion authority.

## Ask component-specific acceptance questions

| Mutation boundary | Required review question | Evidence to retain |
| --- | --- | --- |
| Capture | Were roots read back and revision/inventory fences current before final commit publication? | Capture receipt, commit, adapter observations |
| Restore | Are closure and exact cohort compatibility separate from current activation admission? | Closure report, restore plan, descriptor/cohort evidence |
| Branch | Does the successor satisfy purpose, parents, and one-step generation fencing? | Detached claim, signer observations, current authority evidence |
| Merge | Are the base and every source verified, conflicts resolved by an admitted rule, and generated roots persisted before commit publication? | Source commits, exact profile, conflict or result artifacts |
| Promotion | Is the complete reservation set coupled to the active-head transition? | Promotion plan, reservation set, persistence classification |
| Dispatch | Are handler, adapter, generation, capability, policy, and authority rechecked? | Reservation, attempt, bound outcome observation |

The [head contract](../../world-branch-heads.md) limits generation fencing to intact durable state; it does not establish rollback detection without an independent witness. Promotion eligibility is not external completion, and a retry is not exactly-once execution.

## Review discrepancies without inventing guarantees

- [ ] Has the reviewer separated stated contracts from the helper actually called? The [merge reducer](../../../crates/molten-core/src/world_merge/admission.rs) has an equal-root shortcut before later per-root checks. Require the missing admission/material evidence rather than assuming the diff classifier ran.
- [ ] Is transaction timing described accurately? [Promotion storage](../../../src/world_promotion/store/transaction.rs) validates supplied transaction facts before opening the write transaction, then compares the head inside it. Do not claim all authority observations are acquired transactionally.
- [ ] Are snapshot planning flags treated as supplied facts rather than live authority? Standalone restore still needs an admitted runtime adapter, as documented by the [snapshot contract](../../world-execution-snapshots.md).

These are source-review observations, not reproduced failures. Record each unresolved difference against the exact affected claim.

## Worked rejection and decision wording

A request previews generation 11 with an exact matching plan reference, but the embedding observes generation 12 before checkpoint. Reject the mutation against that preview. Preserve earlier inspection evidence and the drift blocker; do not mark checkpoint or subsequent promotion complete. Require current observations and a new review, not an edited generation attached to old approval.

Record one bounded decision: planning-only accepted, mutation admission evidenced for the named composition, denied with its first blocker, or uncertain pending reconciliation. Include evidence and non-claims. Unknown promotion does not justify automatic retry; absent acknowledgment does not prove failure.

## Sources

- [Handbook](../README.md)
- [World workflow contract](../../world-operator-workflows.md)
- [World branch-head contract](../../world-branch-heads.md)
- [Promotion and effect-release contract](../../world-promotion-and-effect-release.md)
- [Preview-first composition companion](../../technical/world-effects/preview-first-operator-composition.md)
- [Operator service and fresh admission](../../../src/world_operator/service.rs)
- [Promotion transaction store](../../../src/world_promotion/store/transaction.rs)
- [Logical workflow request fixture](../../../tests/fixtures/world-operator/logical/request.json)
