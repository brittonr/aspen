# Inspecting pins before a GC plan

Mode: How-to

## Goal and prerequisites

Prepare a candidate dossier that distinguishes a known retaining relationship from missing knowledge before asking the retention workflow to plan a destructive operation. The result is an inspection record and a decision to continue gathering evidence or hand it to the authorized planning owner—not permission to delete.

You need the correct existing retention root, a canonical object reference obtained from actual inventory, and the owner of that object's retention policy. You also need a fresh destination for an explanation artifact. Commands are source-checked and were not executed for this handbook batch. No build, installed binary, live peer availability, or current authority is implied.

## 1. Start broad enough to find competing evidence

Inspect by object reference first. The optional object-kind, class, action, and subsystem filters are useful later, but prematurely setting them can hide evidence from a different operation. The explanation implementation collects matching pins, admissions, clearances, imports, plans, applies, executions, audits, receipts, and tombstones. It does not discover every remote dependency or certify a complete world inventory.

This invocation is supported by the [root declaration](../../../src/main/root/parts/command/p000/body.rs), [subcommand declaration](../../../src/cli/workflow/retention/command.rs), [Explain arguments](../../../src/cli/workflow/retention/command/ops.rs), and [handler](../../../src/cli/workflow/retention/ops.rs), routed through [main aliases](../../../src/main.rs) and the [root dispatcher](../../../src/main/root.rs). It is source-checked, not executed. Supply real values; the guards deliberately provide no sample authority references:

```sh
: "${RETENTION_ROOT:?Set the existing retention root}"
: "${OBJECT_REF:?Set a canonical reference from actual inventory}"
: "${EXPLAIN_OUT:?Set a fresh explanation artifact path}"
test -d "$RETENTION_ROOT" && test ! -e "$EXPLAIN_OUT" &&
  molten test retention explain --root "$RETENTION_ROOT" \
    --object-ref "$OBJECT_REF" --out "$EXPLAIN_OUT"
```

This reads the candidate evidence graph and writes the requested output artifact; it is not an admission or destructive evaluation. Preserve the structured output and its source root context rather than relying only on the printed counts.

## 2. Decide whether an observed pin is current protection

For each returned pin, inspect object, kind, class, source, reason, owner, expiry reference if present, and supporting policy/evidence references. Ask the owning subsystem whether the underlying session, replay, hold, or other dependency is still active. An expiry reference is not a license to assume wall-clock expiry or manually remove the record.

If pins remain active, stop planning destructive work for that candidate. If a pin looks stale, escalate the specific record to its owner and retain both observations. Do not edit the store, delete a pin file, or run unpin merely to make the candidate eligible. A historical pin artifact in a diagnostic bundle is also not necessarily an active pin; distinguish saved evidence from records currently collected from the store.

## 3. Decide whether absence means empty or unknown

No local pin does not establish a complete reference index. For world objects, obtain explicit observations for all closed root classes: heads, competing heads, execution and task state, replay/simulation/comparison pins, conflicts, promotion/reconciliation state, holds, remote leases, and incomplete transfers.

The [world projection](../../../crates/molten-core/src/world_distribution/retention.rs) requires observed classes, resolved remote observations, complete edges, and complete attribution inventory. An observed-empty class differs from an omitted class. Record missing classes as blockers, not as zero roots. The [handoff](../../../src/world_distribution/retention.rs) combines world completeness with existing retention completeness using conjunction; a complete world report cannot repair an incomplete wider index.

## 4. Separate remote facts from remote permission

For named remote relationships, identify peer observations and their clearance evidence. Uncertain, contradictory, and unavailable lease observations retain roots and block completeness under the distribution contract. A cleared lease removes that lease's retaining contribution; it does not cancel a replay pin or a legal hold.

Do not substitute a replication receipt, successful transfer, plan reference, or benchmark deletion count for clearance. The retention tests explicitly cover a GC plan presented as clearance and expect denial. If current remote evidence cannot be obtained, retain the candidate and stop the destructive path.

## 5. Hand off only an honest dossier

Provide the planning owner with object identity, intended action/subsystem, relevant pin records, observed-empty and missing classes, remote dependencies, policy and authority admission references, supporting evidence, and index evidence. Preserve unresolved items explicitly. A future `gc-plan` call stores a dry-run plan; it is not a deletion operation, but it is also not a read-only inspection because it persists the plan.

Worked case: the broad explanation finds no active pin, but replay-class observation is absent and a remote lease is unavailable. The correct outcome is “candidate unproven,” not “unused.” Gather the missing replay observation and resolve the lease through its owner. Until then, do not assert index completeness or manufacture admissions. Consult the [troubleshooting guide](diagnosing-uncollectable-and-unproven-objects.md) if local explanation and wider inventory seem inconsistent.

## Sources

- [Handbook](../README.md)
- [World distribution contract](../../world-distribution.md)
- [Technical companion](../../technical/world-effects/distribution-retention-and-reachability.md)
- [Candidate explanation collector](../../../src/retention/parts/mod/p038/body.rs)
- [World projection](../../../crates/molten-core/src/world_distribution/retention.rs)
- [Plan is not clearance regression](../../../src/retention/parts/mod/tests/m000/p001/body.rs)
