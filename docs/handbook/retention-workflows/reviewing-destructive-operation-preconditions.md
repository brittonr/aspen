# Reviewing destructive-operation preconditions

Mode: Review checklist

Use this checklist when reviewing a proposed retention operation or a claim that one completed. Record the evidence reference and owner beside each answer. “Not observed” is a blocking answer where evidence is required, not an invitation to infer success from a receipt count.

This checklist is source-checked, not an executed review or runtime certification. It covers retention's declared evidence boundaries; it does not prove that an arbitrary caller obtained truthful observations or that an external deletion adapter erased every copy. Consult the [technical companion](../../technical/world-effects/distribution-retention-and-reachability.md) for theory and the [reference](retention-root-and-deletion-evidence-reference.md) for artifact families.

## Candidate and scope

- [ ] Is the candidate identified by a canonical reference from actual inventory, with object kind, retention class, intended action, and owning subsystem recorded? Reject mutable names, fixture labels, and copied sample references as identity evidence.
- [ ] Is the requester identified, and do policy, authority, supporting-evidence, and reference-index admissions bind that requester and the same object/kind/class/action? Inspect the records, not only the lists containing their references.
- [ ] Are admission decisions passing, currentness asserted by the appropriate owner, revoked-reference lists handled, and bound references nonempty? The admission checks inspect these properties; their presence does not independently prove a producer's current authority.
- [ ] Is the action valid for the class? The evaluator denies destructive actions for `legal-hold` and compaction for `private-secret-ref`. A class-profile convenience field must not be used to override those checks.

Acceptance evidence is a scoped candidate dossier, not a generic statement that “GC is enabled.” A policy for another object or requester is not transferable permission.

## Retaining relationships and completeness

- [ ] Does the active pin inventory agree with the proposed candidate? For each pin, is the source/reason/owner understood? A saved historical pin artifact and a currently stored pin must be distinguished.
- [ ] Are known retained dependencies absent from deletion candidates? If one remains, has the proposal stopped rather than changed the input list to hide it?
- [ ] For world objects, are current and competing heads, execution/task state, replay/simulation/comparison pins, merge conflicts, promotion/reconciliation state, holds, remote leases, and incomplete transfers explicitly observed?
- [ ] Are observed-empty classes recorded as such, rather than reconstructed from omitted observations? Are edge and attribution inventories complete too?
- [ ] Does completeness remain true after the world handoff combines the world report with the wider retention index? The handoff uses conjunction, so a narrower complete report cannot repair missing wider evidence.

Acceptance evidence includes the complete observation boundary and remaining blockers. An empty local explanation, successful replication, or new active head is insufficient.

## Remote clearance and uncertainty

- [ ] Does each required remote relationship have appropriately scoped evidence and clearance coverage? Have peer context and remote references been preserved rather than flattened into a single “remote OK” assertion?
- [ ] Are uncertain, contradictory, and unavailable remote leases treated as retaining/blocking observations? Is any claimed cleared lease supported by its owner rather than inferred from a timeout?
- [ ] Have plan references, replication receipts, benchmark receipts, and supporting observations been kept out of authority/clearance roles they do not own?
- [ ] If an external operation may already have happened, is reconciliation planned before another effect attempt? The review must not authorize unconditional retry of an uncertain operation.

Acceptance evidence is the relevant clearance chain, not the mere availability of a second replica. The checked-in regression that rejects a plan used as clearance is a useful source-review anchor; it was not executed for this batch.

## Planning, apply, and effect boundary

- [ ] Does the stored dry-run plan expose its gate decisions and diagnostics? A successful CLI exit that wrote a denied plan is not a passing plan.
- [ ] At apply, does recomputation preserve the original plan reference, and do admission and evaluation still pass? Record any `retention-gc-apply-plan-drift` instead of dismissing it as cosmetic.
- [ ] Is the actual destructive subsystem identified? The retention CLI provides planning, apply, and audit commands, but no generic `gc-execute` subcommand in its enum. Do not invent one or describe apply's tombstone as physical deletion.
- [ ] For a completion claim, are plan, apply, execution, and audit present, passing, and scope-consistent? Check plan and recomputed-plan links, execution-to-apply links, and receipt/tombstone links—not just artifact existence.

Acceptance evidence for planning is deliberately weaker than evidence for completed effects. Keep those review outcomes separate.

## Worked review rejection

A proposal includes a complete world reachability report, a passing benchmark row with planned deletions, and an old passing GC plan. A new operator hold now pins the candidate. Reject the destructive proposal: the hold is a known retaining dependency, the benchmark grants no authority, and apply must recompute against current state rather than reuse the old plan's conclusion.

Return the exact blocking pin and owner, the proposed plan reference, and the unresolved currentness question. Do not suggest removing the hold, changing class, manufacturing admissions, or deleting state. A later owner-authorized change requires fresh evidence and review; it does not retroactively validate the rejected proposal.

## Sources

- [Handbook](../README.md)
- [World distribution contract](../../world-distribution.md)
- [Benchmark retention limits](../../world-benchmark-sharing-and-retention.md)
- [Technical companion](../../technical/world-effects/distribution-retention-and-reachability.md)
- [Admission scope checks](../../../src/retention/parts/mod/p011/body.rs)
- [World handoff](../../../src/world_distribution/retention.rs)
- [Apply recomputation](../../../src/retention/parts/mod/p014/body.rs)
- [Lifecycle linkage checks](../../../src/retention/parts/mod/p017/body.rs)
- [Retention CLI enum](../../../src/cli/workflow/retention/command.rs)
- [Plan and clearance regressions](../../../src/retention/parts/mod/tests/m000/p001/body.rs)
