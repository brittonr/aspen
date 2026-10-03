# Upgrade task and compatibility reference

Mode: Reference

Use this reference to interpret an existing upgrade plan and its receipts, not to infer deployment support from task names. Entries describe inspected source; no runtime verification was performed for this batch. Canonical Preserves values and BLAKE3 references define identity, while Rust fields are an API view. Receipts describe decisions and evidence; they do not grant authority.

## Plan fields and review ownership

The [input types](../../../src/upgrades/parts/mod/p000/body.rs), [plan construction](../../../src/upgrades/parts/mod/p006/body.rs), and [validators](../../../src/upgrades/parts/mod/p003/body.rs) are the implementation owners. “Reviewer” below means the evidence owner a human review needs, not an additional schema field.

| API field group | Meaning | Reviewer responsibility |
| --- | --- | --- |
| `session_id`, `reason`, `summary` | Session naming and declared intent | Explain the requested change without treating names as artifact identity |
| `initiator_ref`, `capability_refs`, `policy_refs` | Bound initiating and admission references | Resolve current authority through its actual governing boundary |
| `affected_refs` | Explicit change subjects | Reconcile old and new artifacts with task bindings |
| `impact_refs` | Impact population included in the plan | Identify registry versus ledger discovery and scope omissions |
| `tasks` | Ordered `UpgradeTaskInput` values | Review each task's implemented effect, not just its kind |
| `compatibility` | Old/new sets, optional expiry, policy refs | Verify direction and intended lifetime independently |
| `rollback_refs` | Bound rollback strategy references | Prove retained artifacts and permitted reversal semantics |
| `evidence_refs` | Supporting references | Resolve relevant observations; presence is not verification |
| `source_gate_receipt_values` | Actual receipt values supplied to construction | Distinguish strict validation inputs from synthetic test evidence |

A task carries `task_id`, `kind`, `subject`, optional `from_ref` and `to_ref`, precondition and postcondition reference lists, and `reversible`. Shape validation is separate from task execution. Several kinds require both references; `migrate-storage` additionally rejects a reversible claim. `cleanup` needs at least one artifact reference.

## Task implementation matrix

The [dispatcher](../../../src/upgrades/parts/mod/p007/body.rs) and [task helpers](../../../src/upgrades/parts/mod/p002/body.rs) own this matrix.

| Kind or group | Inspected behavior | Do not infer |
| --- | --- | --- |
| `compatibility-alias` | Writes an alias pointer to `to_ref` | New artifact installation or protocol compatibility |
| `move-name` | Checks an existing pointer against `from_ref`, then writes the target pointer | A live service replacement |
| `transcript-rerun` | Requires nonempty precondition references | Execution or resolution of a transcript receipt |
| `cutover` | Evaluates exact refs, plan evidence, and prerequisite completion | An adapter invocation or atomic distributed switch |
| `migrate-schema`, `migrate-storage` | Require target/recipe and pre/post evidence presence | Executing a transformer or checking application semantics |
| `drain-sessions` | Resolves protocol gates from the ledger and evaluates bindings | Authority or live transport quiescence beyond supplied evidence |
| `install-artifact`, `replace-artifact`, `deprecate` | Check reference/evidence presence | Installing, replacing, or deleting payloads in this helper |
| `install-protocol-bridge` | Checks protocol refs and evidence presence | Starting a bridge process |
| `update-policy`, `update-handler-profile`, `update-handler-policy` | Check both refs and supporting evidence | Activating policy in another subsystem |
| `update-docs`, `rollback-pointer` | Return task-admission checks | Editing documentation or moving a pointer on forward execution |
| `cleanup` | Requires retention/impact evidence and calls cleanup admission | Deleting retained content |

These distinctions prevent completion receipts from being misread as comprehensive orchestration. External execution evidence must name the separate component that performed an effect.

## Compatibility and drain bindings

`UpgradeCompatibilityWindow` has `old_refs`, `new_refs`, `expires_at: Option<u64>`, and `policy_refs`. Validation requires nonempty old/new/policy sets and forbids overlap between old and new. The inspected cutover and drain evaluators do not compare `expires_at` with a clock. An expiry field is therefore not proof of temporal freshness enforcement at these helpers.

For a drain, expected protocol selection is ordered: explicit `from_ref`; otherwise a canonical-reference `subject`; otherwise compatibility old refs; finally affected refs if that set is empty. Explicit from/to references must also appear in affected refs and in the corresponding old/new compatibility sets.

The [drain evaluator](../../../src/upgrades/parts/mod/p008/body.rs) requires a gate with decision `pass`, nonempty session IDs and terminal-state refs, and a matching old protocol. It also requires no diagnostics. The shell attempts to parse every deduplicated task precondition/postcondition reference as a protocol-session gate; arbitrary review references mixed into those lists can deny the drain.

## Artifact and state index

| Artifact/state | Owner and purpose |
| --- | --- |
| `upgrade-plan-v1` | Upgrade planner; immutable session description |
| `upgrade-receipt-v1` | Session/task/rollback/admission operation evidence |
| `upgrade-name-pointer-v1` | Upgrade store; mutable name or alias metadata |
| `protocol-session-gate-receipt-v1` | Protocol-session producer; ledger-resolved drain evidence |
| `status` entries | Upgrade store; receipt references, not editable checkboxes |
| `plans`, `names`, `status` snapshot | No-mutation comparison scope; excludes newly written diagnostic receipts |

[Stored status validation](../../../src/upgrades/parts/mod/p010/body.rs) checks the receipt hash, passing decision, plan reference, and task ID. Denied task receipts are stored but do not become completion status.

## Worked interpretation

A `rollback-pointer` task marked complete does not mean a pointer moved: its forward dispatcher branch only reports admission checks. By contrast, invoking the separate rollback API may write metadata for eligible pointer-related kinds. That API always denies `migrate-storage`, `cleanup`, and `install-protocol-bridge`, regardless of a reversible label. Review the operation and effect owner together.

Numeric bounds include 1,024 tasks, 4,096 refs, 4,096 diagnostics, and 128 source-gate values in the upgrade module. They bound local processing, not workload throughput or readiness. System-extension generation replacement remains the separate mechanism explained in the technical companion.

## Sources

- [Handbook](../README.md)
- [Architecture](../../architecture.md)
- [Upgrade and migration companion](../../technical/extensions/upgrade-quarantine-and-migration.md)
- [Upgrade types and bounds](../../../src/upgrades/parts/mod/p000/body.rs)
- [Validation and compatibility](../../../src/upgrades/parts/mod/p003/body.rs)
- [Cutover evaluator](../../../src/upgrades/parts/mod/p004/body.rs)
- [Drain enforcement](../../../src/upgrades/parts/mod/p008/body.rs)
