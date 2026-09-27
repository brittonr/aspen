# Diagnosing blocked promotion and merge

Mode: Troubleshooting

Start with the first blocked boundary, not the last command in the desired workflow. This guide distinguishes request rejection, unavailable standalone adapters, component conflicts, and uncertain publication. It is source-checked; no scenario or command was executed for this batch. See the [Handbook](../README.md), [promotion companion](../../technical/world-effects/promotion-and-reservation-atomicity.md), and [merge companion](../../technical/world-state/typed-diff-and-merge-conflicts.md) for supporting context.

## Symptom: the preview exists but apply is denied

**Discriminating evidence:** identify whether the invocation was the workflow family or a component family. In the workflow [output path](../../../src/cli/runtime/world/output.rs), exact submitted preview identity produces `HandlerUnavailable`; a different identity produces `StalePlan`. Neither establishes an attempted component mutation. The plan and summary can already have been written before the apply denial overwrites the receipt output.

**Safe next action:** preserve the request, preview, denial receipt, and invocation context separately. Check the exact preview reference, not a component plan reference or a filename. If stale, obtain new observations and review a new preview rather than patching only the submitted reference. If handler-unavailable, identify the missing reviewed embedding.

**Stop condition:** the standalone CLI has no live registry. Repeating apply, signing more artifacts, or editing admission booleans cannot compose one.

## Symptom: promotion planning denies before storage changes

**Discriminating evidence:** the [promotion request adapter](../../../src/cli/runtime/worldpromotion.rs) reads its own JSON shape, including expected and candidate heads, generation, policy, authority facts, intent closure completeness, simulation status, and typed intents. This is not the workflow JSON document. The [promotion contract](../../world-promotion-and-effect-release.md) distinguishes incomplete closure, unclassified intent, simulated branch, stale facts, and reservation mismatch.

**Safe next action:** compare the candidate's complete intent inventory with the request supplied by its owner. Trace every intent to its semantic operation, handler, and adapter. Establish which fact was absent or denied; do not turn a false field true solely to satisfy planning.

**Stop condition:** without evidence-backed intent closure and current adapter composition, a canonical plan is not permission to promote. Standalone `promote` remains disabled after successful planning.

## Symptom: promotion may have committed, but completion is unknown

**Discriminating evidence:** separate local persistence uncertainty from external acknowledgment loss. The [transaction store](../../../src/world_promotion/store/transaction.rs) maps final Redb commit errors to `OutcomeUnknown`; it does not infer non-publication. An external adapter may also have acted before its acknowledgment was lost. Those cases require different observations.

The following source-checked diagnostic commands were not executed here. Their spelling and non-mutating operator behavior are established by the [promotion CLI](../../../src/cli/runtime/worldpromotion.rs), [main aliases](../../../src/main.rs), and [top-level registration](../../../src/main/root/parts/command/p000/body.rs). Supply the existing reviewed state root; do not substitute a fresh empty store.

```sh
molten world-promotion outbox-inspect \
  --state-root "${WORLD_STATE_ROOT:?Set the existing reviewed state root}"
molten world-promotion reconcile \
  --state-root "${WORLD_STATE_ROOT:?}"
```

**Safe next action:** retain the exact plan, reservation, attempt, and persistence evidence for the component's observation-first reconciliation. The CLI reconciliation command reports unresolved reservations; it does not perform an automatic repair or retry. An empty unresolved count is not proof that an external effect completed.

**Stop condition:** unknown, conflicting, corrupt, or incomplete persistence must not be collapsed into safe-to-repeat work. Retry requires explicit duplicate-risk acknowledgment and a new attempt identity while preserving the logical reservation identity. This guide supplies no retry recipe because the standalone current-plan and authority adapters are unavailable.

## Symptom: a merge plan contains conflicts or refuses a base

**Discriminating evidence:** distinguish missing/ambiguous ancestry, unavailable root material, schema disagreement, and genuine concurrent changes. The [merge declaration and loader](../../../src/cli/runtime/parts/worldmerge/p000/body.rs) take explicit base, left, and right commits; the CLI walks ancestry for the supplied base. The pure planner receives common-ancestor facts rather than independently discovering history.

**Safe next action:** retain all source identities and the exact profile. For a durable-key delete-versus-update case, preserve the conflicting key and three versions for application review. Do not pick the newer timestamp or lexically smaller digest. For divergent tasks, scheduler, effects, time, entropy, authority observations, or opaque machine roots, do not substitute keyed application merging.

**Stop condition:** unresolved conflicts prevent merge-commit publication. Even a conflict-free plan does not move a branch; standalone merge publication lacks authority, migration, and handler composition.

## Symptom: diff and merge appear to disagree

**Discriminating evidence:** inspect which helper produced each result. The [merge reducer](../../../crates/molten-core/src/world_merge/admission.rs) selects equal left/right roots with equal schema references before per-root schema-admission and declared-mode checks. The conservative diff contract cannot be assumed to govern that shortcut automatically. Separately, `conflict-inspect` strictly decodes canonical Preserves and prints it; the inspected CLI function does not establish a typed conflict-schema check.

**Safe next action:** record the exact caller and supplied facts for source review, linking the [merge contract](../../world-state-diff-and-merge.md). Do not report a reproduced bug from this documentation-only inspection or treat generic canonical decoding as semantic validation.

**Stop condition:** with unresolved contract/helper differences, narrow the claim to the checks actually evidenced. Likewise, promotion's transaction-fact validation occurs before `begin_write`, although head comparison occurs inside the transaction. Atomic head-and-reservation publication does not prove that every authority observation was acquired within that transaction.

## Sources

- [Handbook](../README.md)
- [World promotion contract](../../world-promotion-and-effect-release.md)
- [World merge contract](../../world-state-diff-and-merge.md)
- [Promotion companion](../../technical/world-effects/promotion-and-reservation-atomicity.md)
- [Typed merge companion](../../technical/world-state/typed-diff-and-merge-conflicts.md)
- [Promotion transaction implementation](../../../src/world_promotion/store/transaction.rs)
- [Merge CLI include implementation](../../../src/cli/runtime/parts/worldmerge/p000/body.rs)
- [Workflow apply-denial output](../../../src/cli/runtime/world/output.rs)
