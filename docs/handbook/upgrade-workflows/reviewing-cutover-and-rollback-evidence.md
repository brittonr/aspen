# Reviewing cutover and rollback evidence

Mode: Review checklist

Use this checklist at a review handoff for one exact upgrade plan. Record the reviewer, plan identity, evidence locations, and unresolved conditions in the team's existing review system. This page does not define another approval artifact or replace the [architecture's authority boundaries](../../architecture.md). It is based on source inspection; no upgrade, rollback, smoke run, or test suite was executed for this batch.

A review should distinguish four results: a valid plan, admitted task completion, observed external effects, and admissible recovery. A receipt in one category is not evidence for all four. The [technical companion](../../technical/extensions/upgrade-quarantine-and-migration.md) supplies the theory for generation replacement and directional migration; the questions below make the operational evidence obligations explicit.

## Candidate and plan identity

- [ ] Can another reviewer identify the exact old and new canonical artifacts without resolving a mutable name? Retain content refs and the source of the corresponding bytes. Rust layout and debug rendering are not identity inputs.
- [ ] Does the session describe the intended change, including every affected protocol, schema, storage surface, and policy surface? Compare `affected_refs` with task from/to refs; do not silently narrow the change to a name move.
- [ ] Is impact evidence attributable to its discovery mechanism? The [name-move planner](../../../src/upgrades/parts/mod/p001/body.rs) has ledger and registry paths. Record which population was inspected and which external consumers remain outside it.
- [ ] Are source-gate receipt values genuine evidence for the reviewed candidate? [Plan construction](../../../src/upgrades/parts/mod/p003/body.rs) validates receipt values, while fixtures and the rewrite integration hook may supply synthetic gates. A construction success using test evidence is not release approval.

## Ordering and actual effects

- [ ] Is every prerequisite completion backed by a matching stored receipt? The [status reader](../../../src/upgrades/parts/mod/p010/body.rs) requires a matching hash, passing decision, plan reference, and task ID. Preserve both receipt and status evidence rather than screenshots of a remaining-task count.
- [ ] Has the reviewer identified the first actual mutation? In the generated name-move plan, alias creation and `move-name` occur before the final cutover task. Do not state that an incomplete cutover proves unchanged routing metadata.
- [ ] Do the named tasks actually invoke the effect claimed by the change request? The [dispatcher](../../../src/upgrades/parts/mod/p007/body.rs) checks evidence presence for several kinds. `update-docs` and forward `rollback-pointer` do not implement those named effects; `transcript-rerun` does not run a transcript.
- [ ] For each external effect, is there evidence from the component that performed it? Artifact installation, application migration, live protocol operation, and system-extension replacement need their own observations. An upgrade coordination receipt cannot fill these gaps.

## Cutover prerequisites and their limits

- [ ] Does the [cutover evaluator](../../../src/upgrades/parts/mod/p004/body.rs) have exact from/to refs, impact refs, compatibility sets and policy, policy/capability refs, supporting evidence, rollback refs, and a completed transcript task? Inspect the values, not only check names.
- [ ] If schema or storage migration tasks exist, are they complete, and does separate evidence show what transformed the application state? The helper checks reference presence rather than executing a migration.
- [ ] If a protocol drain task exists, does its ledger-resolved gate pass for the old protocol and bind nonempty sessions and terminal states? Reconcile affected and old/new compatibility bindings. Do not infer that absent drain tasks prove draining was unnecessary; review the requested change's protocol scope.
- [ ] Has temporal compatibility been reviewed separately? The window records optional `expires_at`, but the inspected cutover and drain helpers do not evaluate it against time. List the policy and observation that justify current freshness.
- [ ] Have denied attempts preserved their evidence? The no-mutation comparison covers plans, names, and status, not all possible adapter effects or newly written receipts. A negative check requires investigation before additional mutation.

## Rollback eligibility and reconciliation

- [ ] Does the recovery proposal distinguish metadata reversal from undoing external effects? [Rollback implementation](../../../src/upgrades/parts/mod/p002/body.rs) always denies storage migration, cleanup, and protocol-bridge kinds, and also denies tasks not marked reversible.
- [ ] For eligible pointer-related tasks, is `from_ref` available and still the intended destination? The rollback helper writes selected pointer metadata; it does not prove semantic recovery or automatically reconcile every prior effect.
- [ ] Will the post-rollback review inspect current pointers as well as completion receipts? The rollback helper stores its receipt without clearing task completion status. Old completion records and current routing can therefore describe different stages of the history.
- [ ] For system-extension recovery, are active generation, manifest, checkpoint, and destination schema compatibility inspected independently? Rollback is a new generation, not a decrement to an earlier generation. Retained old executable bytes alone do not establish the reverse migration direction.
- [ ] Are retention and cleanup decisions separate from rollback eligibility? The name-move fixture denies old-artifact cleanup after all four tasks pass. Never delete evidence or retained state to make a checklist appear complete.

## Worked review rejection

A proposal supplies a passing name-move session and says “rollback is guaranteed because the old module remains.” Accept the narrow observation that metadata tasks completed, if their receipts resolve. Reject the broader guarantee until the reviewer identifies the actual effect surface and admissible recovery path.

If this was only a pointer change, inspect the pointer and reversal evidence. If a storage transformation happened elsewhere, the upgrade rollback helper explicitly does not undo it. If a system extension was replaced, require directional schema admission and observed recovery for the active state. Record these as missing evidence, not as a reproduced runtime defect. Approval should name the supported boundary and leave no ambiguous claim of automatic rollback, exactly-once execution, or production readiness.

## Sources

- [Handbook](../README.md)
- [Architecture](../../architecture.md)
- [Upgrade, quarantine, and migration theory](../../technical/extensions/upgrade-quarantine-and-migration.md)
- [Cutover readiness and no-mutation scope](../../../src/upgrades/parts/mod/p004/body.rs)
- [Task effects and rollback](../../../src/upgrades/parts/mod/p002/body.rs)
- [Receipt-backed status](../../../src/upgrades/parts/mod/p010/body.rs)
- [Name-move and irreversible rollback fixtures](../../../src/upgrades/parts/mod/tests/m000/p000/body.rs)
- [Rewrite upgrade integration](../../../src/rewrites/parts/mod/p001/body.rs)
