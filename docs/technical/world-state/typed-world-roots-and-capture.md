# Typed World Roots and Capture

A world commit names a profile-relative snapshot, not an undifferentiated dump of everything a process can access. This article explains how typed roots, canonical identity, and revision fences cooperate without transferring subsystem ownership to the snapshot mechanism. Read the governing [world-commit contract](../../world-commit.md) first; the [Technical companion](../README.md) provides the wider documentation context.

## A root is a claim about a domain

The closed root vocabulary separates artifacts and schemas from durable values, tasks, history, effects, scheduler state, time, entropy, runtime profile, policy, authority observations, and opaque machine snapshots. A digest alone cannot replace this classification. The capture core compares an observation's declared source domain with the typed root domain, and rejects mismatches rather than interpreting a task object as a durable-state object merely because both have valid-looking references.

Completeness is relative to the selected profile. Logical capture requires the logical runtime roots and excludes opaque machine state. Opaque capture requires an exact cohort and opaque machine snapshot while excluding the logical runtime roots. Mixed capture requires both. Artifact, schema, runtime-profile, and policy roots remain required across the profiles. An authority observation is optional historical evidence, never a transported live grant. These distinctions determine what can subsequently be verified, replayed, or restored; they are not interchangeable storage optimizations ([profile table](../../world-commit.md#typed-roots)).

The immutable commit contains its schema/version, profile, parents, typed roots, and completeness declaration. Signatures, annotations, mutable branch heads, and currentness facts are outside that core. Canonical packed Preserves and framed, domain-separated BLAKE3 identity therefore commit to protocol meaning rather than Rust memory layout. Normalizing input order does not erase distinctions between root domains or profiles.

## What the capture plan actually establishes

In [capture.rs](../../../crates/molten-core/src/worldcommit/capture.rs), `RootObservation` carries `source_kind`, `schema_validated`, `stability`, `durable`, and `inventory_complete` alongside the root. The booleans are supplied observations: pure planning does not open an object store or execute a schema adapter to discover them. Adapter correctness remains a separate premise.

For mutable material, `RevisionFence` binds a root kind, source identifier, and observed revision. `plan_capture` validates collection bounds, rejects duplicate fence sources, normalizes the candidate core, and returns the roots requiring persistence plus the fences requiring recheck. Its determinism concerns supplied facts and returned plans; it does not turn independent stores into a shared transactional snapshot service.

The [shell](../../../src/worldcommit/shell.rs) supplies the operational sequence:

1. Observe requested root material and consult storage for durability.
2. Invoke the pure planner before attempting root publication.
3. Persist missing roots and read all declared roots back.
4. Verify canonical bytes and their content references.
5. Collect revision and inventory rechecks, then compare them with the plan.
6. Canonicalize and publish the commit only after those checks succeed.

The final commit publication is the capture's last mutation. Root writes preceding it are immutable material, not proof that a coherent capture has completed. A publication error is represented as uncertain rather than promoted into a successful commit identity.

## Worked failure: a complete object set, an incoherent cut

Consider an illustrative logical capture with durable-state revision 41 and tasks revision 12. All required root objects exist, decode canonically, and match their references. While the shell persists another missing root, the task owner accepts a new task and advances to revision 13.

The task recheck now disagrees with the fence. `compare_revision_rechecks` returns a non-current comparison containing revision drift. The shell returns a denied execution with no successful commit. Presence of every object is insufficient: the captured task inventory no longer matches the observed mutable cut.

A second variant preserves revision 12 but reports an incomplete task inventory. That also denies. Treating inventory completeness as synonymous with digest integrity would miss this case: a perfectly intact object may describe only a partial enumeration. A third variant returns two rechecks for one source; duplicate evidence is rejected rather than allowing the later answer to silently replace the earlier one.

These examples explain why capture and closure are separate questions. Closure establishes declared object presence and identity, including bounded parent reachability. It does not retroactively repair a failed capture fence or establish authorization to activate the result.

## Review and verification guidance

Review the revision source contract first: what mutation increments it, and can an adapter omit part of an inventory while declaring completeness? Next trace every root from observation through canonical-byte verification and publication. Finally distinguish a capture receipt, a closure report, and a restore plan when reading operator diagnostics.

The existing [core tests](../../../crates/molten-core/src/worldcommit/tests.rs) contain input-order normalization, domain-confusion, revision-drift, incomplete-inventory, missing-root, and parent-cycle scenarios. Suggested focused checks are `cargo test -p molten-core world_commit` and `cargo test --lib world_commit`; they were not executed for this documentation change. An integration review should additionally interrupt publication and inspect the resulting receipt rather than infer success from root files alone.

## Limits and non-claims

This is a fenced coherent local cut, not atomicity across independent services. Local synchronized files do not establish replication, rollback resistance, or race-free multi-writer publication. Successful closure does not establish compatibility or successful restoration. Restore planning still requires current policy, authority, resources, runtime, and effect admission before activation. Historical authority roots cannot discharge those checks.

## Sources

- [World-commit contract](../../world-commit.md)
- [Branch-head contract](../../world-branch-heads.md)
- [Capture planning and revision comparison](../../../crates/molten-core/src/worldcommit/capture.rs)
- [Capture and restore shell](../../../src/worldcommit/shell.rs)
- [Core capture and closure tests](../../../crates/molten-core/src/worldcommit/tests.rs)
- [Technical companion](../README.md)
