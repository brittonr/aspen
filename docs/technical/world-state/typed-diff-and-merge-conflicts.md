# Typed Diff and Merge Conflicts

Comparing world roots and authorizing a merged world are different operations. Diff describes supplied state; merge interprets changes under an exact profile and retains unresolved disagreements as artifacts rather than selecting a convenient winner. This article assumes the [world-state diff and merge contract](../../world-state-diff-and-merge.md) and [typed world-commit model](../../world-commit.md). See the [Technical companion](../README.md) for adjacent topics.

## Conservative classification preserves missing information

[diff_world_roots](../../../crates/molten-core/src/world_merge/diff.rs) checks the supplied root bound and duplicate root kinds, classifies each supplied root, and orders the result by kind. Classification has deliberate precedence: exclusion by profile first, then unavailable material, roots absent from base, left, and right alike, incompatible schemas among the present roots, and finally equality or change between the left and right root references.

This precedence prevents unavailable objects from being reported as equal merely because their references happen to match. Absence and unavailability also remain distinct: one concerns a missing reference, the other unavailable material. `absent` is reserved for a root missing from all three inputs. Branch creation (absent base, present side) and deletion (present base, missing side) are compared against the present roots: they classify as `changed`, or `incompatible` when the present roots' schemas differ, and a side missing its root is never `equal`, even when both sides deleted it. Two sides creating the same root are `equal`. Schema compatibility in this classifier compares the schema references of the present roots directly; it does not execute a migration to discover whether two schemas could be reconciled.

The function processes the supplied collection. Its output should not be read as proof that an upstream caller supplied every root required by a complete world profile. Nor does an equal classification authorize branch mutation, runtime activation, or effect release.

## Base selection and merge meaning

The governing contract requires one verified common ancestor and rejects ambiguous bases before handler execution. In the [pure planner](../../../crates/molten-core/src/world_merge/admission.rs), `common_ancestor_verified` and `common_ancestor_ambiguous` are explicit input facts checked during request validation. The reducer does not independently walk history to establish them. This distinguishes a deterministic check of supplied evidence from the integration responsibility to obtain trustworthy ancestry observations.

The closed modes are identical-only, ancestor-replacement, keyed-durable-values, and application-handler. Identical-only retains equal state. Ancestor replacement permits one changed side when the other equals the base. Keyed merging is restricted to durable state. Application handlers bind exact behavior, schemas, policy, and bounds and receive loaded bytes; every present base, left, and right root must carry the profile's declared `input_schema` before the handler runs (`HandlerInputSchemaMismatch` otherwise), and an effect request denies the result.

Tasks, scheduler state, effects, time, entropy, authority observations, and opaque snapshots are runtime-sensitive and rejected, whether the two sides diverge or carry equal references, rather than treated like ordinary application records. A history graph can explain where those values came from without supplying a safe composition rule for them.

## Worked three-way key conflict

Consider illustrative durable records with base values `quota=10` and `label=blue`. The left branch changes only `quota` to `12`; the right changes only `label` to `green`. For each key, the [keyed reducer](../../../crates/molten-core/src/world_merge/handlers.rs) applies the same ordered decision:

1. If left equals right, select that value.
2. Otherwise, if left equals base, select right.
3. Otherwise, if right equals base, select left.
4. Otherwise, record a concurrent-key-change conflict.

The example yields `quota=12` and `label=green`. No timestamp or digest ordering is needed. Now let the right branch instead delete `quota`. The left value `12`, right absence, and base `10` are pairwise incompatible under those rules, so `quota` conflicts. Absence is part of the three-way comparison, not an instruction to favor deletion.

The union of keys and the number of conflicts are bounded. Conflicts are ordered by root kind, optional key, and code in the plan. Deterministic ordering makes artifacts reproducible; it does not select a resolution. An unresolved conflict prevents merge-commit publication even if other keys merged cleanly.

## Schema materialization and publish-last behavior

The [shell](../../../src/world_merge/service.rs) loads root bytes and asks the migration port to materialize admitted source-to-target conversions before invoking the pure planner. A migration binding is not executable evidence by itself. The shell currently supports at most one application-handler profile in a plan; this is an inspected implementation limit, not a universal property of semantic merge.

During publication, conflicts take the conflict-persistence path and return no result commit. Otherwise the shell rechecks merge authority, persists generated roots, verifies returned identities, and only then asks the commit port to publish with the declared source heads. Existing selected roots are reused. A root publication failure can leave unreferenced immutable objects but does not reach the later commit publication call.

The authority recheck occurs before the generated-root loop, not as a second check immediately after that loop. Thus “recheck before publication” should not be expanded into an uninspected atomic transaction spanning authority, root storage, and commit publication. Branch-head movement remains a separate operation.

## Equal roots follow the same admission order

The [governing prose](../../world-state-diff-and-merge.md) says merge is admitted by an exact profile and unavailable material is not equal. The planner's `reduce_root` checks runtime sensitivity, material availability and schema admission, and the declared profile mode before it selects an equal left/right root, so equal references cannot carry a runtime-sensitive root (`Tasks`, `Entropy`, and the other runtime kinds), an unavailable root, or a root kind the profile excludes into a plan. An equal root under the application-handler mode is additionally held to that handler's purity, policy, and declared input schema. Diff and merge remain separate implementations; this ordering is enforced by the planner itself, not by assuming that one calls the other.

## Review, verification, and non-claims

Suggested review cases include delete-versus-update conflict, unavailable equal references, absent equal references, an undeclared mode, ambiguous ancestry, a handler requesting effects, and failure while persisting a generated root. Compare canonical conflict/result artifacts with durable publication state; diagnostic output alone is insufficient. These scenarios were not executed for this documentation change.

The standalone `merge-publish` command remains fail-closed without composed adapters. Handler identity does not prove correctness; migration planning does not prove migrated data correctness. Neither successful merge nor conflict storage proves branch authority, effect safety, remote convergence, release eligibility, or production readiness. Determinism makes disagreement inspectable, not automatically resolvable.

## Sources

- [World-state diff and merge contract](../../world-state-diff-and-merge.md)
- [World commits](../../world-commit.md)
- [Conservative root classifier](../../../crates/molten-core/src/world_merge/diff.rs)
- [Merge admission and root reduction](../../../crates/molten-core/src/world_merge/admission.rs)
- [Three-way keyed merge and handler interface](../../../crates/molten-core/src/world_merge/handlers.rs)
- [Loading, migration, conflicts, and publication shell](../../../src/world_merge/service.rs)
- [Technical companion](../README.md)
