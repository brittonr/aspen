# Node lifecycle and recovery

This article explains how to read local node lifecycle evidence without mistaking a filesystem transition for production recovery. It assumes familiarity with canonical Preserves receipts and the [production operator runbooks](../../production-operator-runbooks.md). Those runbooks govern operational review; this article is explanatory, not an alternative recovery procedure. See the [Technical companion](../README.md) for related topics.

## Lifecycle state is a constrained observation

The daemon's `node_lifecycle_state` classifies five observed facts: configuration, identity receipt, startup receipt, shutdown receipt, and active lock. Its [implementation](../../../src/node/parts/daemon/p036/body.rs) recognizes four combinations. An empty root has none of these facts. An initialized root has configuration and identity but neither lifecycle receipt nor lock. A running root additionally has startup and the active lock, but no shutdown. A stopped root has both lifecycle receipts and no active lock. Other combinations are `Inconsistent`.

This is useful precisely because it does not infer intent. A lock without startup is not evidence of a healthy process. A startup receipt without a clean shutdown does not become a stopped node merely because an operator believes the process exited. The classification is a deterministic function of supplied file-presence observations; obtaining those observations and reading durable artifacts are shell effects.

The presence-only inspector is not the current-run status decision. The [status path](../../../src/node/parts/daemon/p018/body.rs) supplies exact run/startup, active-lock and completion observations to the pure lifecycle classifier. A current completion must also name matching passing shutdown and immutable control receipts. Missing, unreadable or conflicting observations reject rather than deriving `stopped` from a filename or lock absence. Status still constructs and imports health/control evidence; its result is not an independent process-liveness or adapter-responsiveness probe.

## Startup and shutdown have different admission boundaries

`run_local_with_root` checks restart state before reading configuration and identity. It gathers runtime-startup refs, writes adapter-start and startup receipts, and rejects a denied startup before installing an active lock. A passing startup publishes canonical run context and a matching run-bound lock, then imports startup evidence. Each run context binds state-root/startup/previous-run refs, so identical startup bytes do not merge successive runs. These are ordered operations, not one demonstrated atomic transaction across every file and ledger member.

Shutdown makes its admission boundary especially visible. `stop_local_node_with_request` validates current observations and calls `admit_shutdown_request`. If no plan is returned, it writes control-denial evidence and exits without executing a shutdown plan. An admitted plan writes/imports adapter, node shutdown and control receipts, publishes exact run/startup/shutdown/control completion linkage, and removes the active lock. The [shutdown regression source](../../../src/node/parts/daemon/tests/m000/p014/body.rs) checks admission-denial preservation; [current-run regressions](../../../src/node/parts/daemon/tests/m000/p023/body.rs) cover observation rejection, historical duplicates and later denied stops.

A receipt that records denial is valuable evidence, but it is not a failed-looking substitute for a successful transition. Similarly, a write failure during an admitted plan is not equivalent to admission denial: earlier effects may already have happened. Recovery analysis should identify the observed boundary rather than describing all errors as interchangeable.

Duplicate shutdown success remains evidence about the original request and run. After
restart it does not execute again or stop a consistent later run, even with identical
startup content. A later denied direct stop may replace the mutable last-control file;
completed-state readback instead resolves the immutable control reference in the exact
completion. Service denial and lifecycle completion are also independent: bounded
pending work can deny a service run while its verified current shutdown remains stopped.

## Worked failure: a process disappears after startup

Consider an illustrative local run that has configuration, identity, startup, and an active lock. The process is interrupted before a shutdown receipt is written. A later run reaches `verify_restart_state`, sees the existing startup, and finds no shutdown. The implementation rejects restart with a diagnostic explaining that the previous startup has no clean shutdown receipt.

Deleting the lock supplies neither the missing shutdown nor its exact current-run completion. Manufacturing a receipt would destroy the meaning of the review. Preserve the root and recorded artifacts, determine what actually completed, and use the separately authorized operational recovery process. This article does not prescribe a bypass or assert that arbitrary interrupted states are automatically repairable.

For a clean prior stop, restart first requires matching current-run completion and no active lock, then constructs restart-health evidence tied to startup, shutdown and current index refs. That decision must pass before the old shutdown/completion files are cleared and a successor run binding is published. This local mechanism does not prove that backups, external services or irreversible migrations can be replayed safely. Legacy receipts remain readable history but cannot supply missing run lineage.

## Recovery evidence extends beyond the daemon

The [runbooks](../../production-operator-runbooks.md) require backup drills to bind ledger, Redb, chunks, identity metadata, retention pins, source-gate refs, restore verification, and tamper-denial evidence. A directory copy alone cannot express those relationships. Upgrade and rollback evidence additionally addresses migration, smoke observations, rollback eligibility, irreversible exclusions, and post-rollback verification. An irreversible operation without exclusion evidence cannot justify a rollback-safety claim.

There is an important source boundary: the inspected local startup uses `synthetic_clean_octet_gate_receipt_for_tests()`. The production runbook, by contrast, requires current evidence for the exact reviewed source candidate and policy. Local lifecycle success therefore does not demonstrate fulfillment of that production requirement. This article deliberately leaves that gap visible rather than treating the local fixture path as production admission.

## Verification and limits

Suggested review follows the runbook's init/run/status/stop sequence in an isolated root and retains startup, health, shutdown, current-run, exact completion and runbook-check artifacts. Compare canonical refs and decisions, including a prior-run duplicate and a new admitted shutdown. Review denied admission separately from observation rejection, aggregate service denial and interrupted writes. For a cluster exercise, the [receipt-first harness](../../receipt-first-cluster-harness.md) preserves child lifecycle and cleanup evidence in its durable run directory.

These are proposed verification activities, not commands executed for this article. No crash-consistency theorem, global liveness claim, automatic stale-state repair, or production readiness result follows from the inspected local mechanics.

## Sources

- [Production operator runbooks](../../production-operator-runbooks.md)
- [Receipt-first cluster harness](../../receipt-first-cluster-harness.md)
- [Daemon startup, status, and shutdown implementation](../../../src/node/parts/daemon/p018/body.rs)
- [Lifecycle classification and restart implementation](../../../src/node/parts/daemon/p036/body.rs)
- [Shutdown preservation regression cases](../../../src/node/parts/daemon/tests/m000/p014/body.rs)
- [Technical companion](../README.md)
- [Canonical run and completion values](../../../src/node/parts/runtime/p009/body.rs)
- [Capability-rooted observations and immutable completion readback](../../../src/node/parts/daemon/p058/body.rs)
- [Current-run regression cases](../../../src/node/parts/daemon/tests/m000/p023/body.rs)
