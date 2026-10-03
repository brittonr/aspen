# Node lifecycle and recovery

This article explains how to read local node lifecycle evidence without mistaking a filesystem transition for production recovery. It assumes familiarity with canonical Preserves receipts and the [production operator runbooks](../../production-operator-runbooks.md). Those runbooks govern operational review; this article is explanatory, not an alternative recovery procedure. See the [Technical companion](../README.md) for related topics.

## Lifecycle state is a constrained observation

The daemon's `node_lifecycle_state` classifies five observed facts: configuration, identity receipt, startup receipt, shutdown receipt, and active lock. Its [implementation](../../../src/node/parts/daemon/p036/body.rs) recognizes four combinations. An empty root has none of these facts. An initialized root has configuration and identity but neither lifecycle receipt nor lock. A running root additionally has startup and the active lock, but no shutdown. A stopped root has both lifecycle receipts and no active lock. Other combinations are `Inconsistent`.

This is useful precisely because it does not infer intent. A lock without startup is not evidence of a healthy process. A startup receipt without a clean shutdown does not become a stopped node merely because an operator believes the process exited. The classification is a deterministic function of supplied file-presence observations; obtaining those observations and reading durable artifacts are shell effects.

Nor is the local status string an independent process-liveness oracle. In the inspected [status path](../../../src/node/parts/daemon/p018/body.rs), `status_local_node_with_request` parses startup, looks for a shutdown receipt, and reports `stopped` or `running` accordingly. It constructs and imports health and control receipts from that evidence. That projection should not be promoted into a claim that every adapter, transport, or cluster participant is currently responsive.

## Startup and shutdown have different admission boundaries

`run_local_with_root` checks restart state before reading configuration and identity. It gathers the refs used by runtime startup, writes adapter-start receipts and the startup receipt, and rejects a denied startup before installing the active lock. A passing startup then writes the lock and imports startup evidence. These are ordered operations, not one demonstrated atomic transaction across every file and ledger member.

Shutdown makes its admission boundary especially visible. `stop_local_node_with_request` obtains the current startup receipt and calls `admit_shutdown_request`. If no plan is returned, it writes control-denial evidence and exits with an error. It does not execute the shutdown plan. When a plan exists, the shell writes and imports adapter-shutdown receipts, writes and imports the node shutdown receipt, writes and imports the control receipt, and finally removes the active lock. The [shutdown regression source](../../../src/node/parts/daemon/tests/m000/p014/body.rs) checks preservation of lifecycle state on denial, including keeping the lock and not publishing shutdown receipts.

A receipt that records denial is valuable evidence, but it is not a failed-looking substitute for a successful transition. Similarly, a write failure during an admitted plan is not equivalent to admission denial: earlier effects may already have happened. Recovery analysis should identify the observed boundary rather than describing all errors as interchangeable.

## Worked failure: a process disappears after startup

Consider an illustrative local run that has configuration, identity, startup, and an active lock. The process is interrupted before a shutdown receipt is written. A later run reaches `verify_restart_state`, sees the existing startup, and finds no shutdown. The implementation rejects restart with a diagnostic explaining that the previous startup has no clean shutdown receipt.

Deleting the lock would not satisfy that check: the missing shutdown receipt remains the relevant evidence gap. Manufacturing a receipt would destroy the meaning of the review. The useful response is to preserve the root and recorded artifacts, determine what actually completed, and use the separately authorized operational recovery process. This article does not prescribe a bypass or assert that arbitrary interrupted states are automatically repairable.

For a clean prior stop, restart hashes the shutdown artifact, constructs restart-health evidence tied to startup and current index refs, writes that health artifact, and requires its decision to pass before removing the old shutdown file. This is the inspected local restart mechanism, not a proof that backups, external services, or irreversible migrations can be replayed safely.

## Recovery evidence extends beyond the daemon

The [runbooks](../../production-operator-runbooks.md) require backup drills to bind ledger, Redb, chunks, identity metadata, retention pins, source-gate refs, restore verification, and tamper-denial evidence. A directory copy alone cannot express those relationships. Upgrade and rollback evidence additionally addresses migration, smoke observations, rollback eligibility, irreversible exclusions, and post-rollback verification. An irreversible operation without exclusion evidence cannot justify a rollback-safety claim.

There is an important source boundary: the inspected local startup uses `synthetic_clean_octet_gate_receipt_for_tests()`. The production runbook, by contrast, requires current evidence for the exact reviewed source candidate and policy. Local lifecycle success therefore does not demonstrate fulfillment of that production requirement. This article deliberately leaves that gap visible rather than treating the local fixture path as production admission.

## Verification and limits

Suggested review is to follow the runbook's init, run, status, and stop sequence in an isolated root, retain startup, health, shutdown, and runbook-check artifacts, and compare their refs and decisions. Review a denied-shutdown case separately from an interrupted-write case. For a cluster exercise, use the [receipt-first harness](../../receipt-first-cluster-harness.md), whose durable run directory preserves child lifecycle and cleanup evidence.

These are proposed verification activities, not commands executed for this article. No crash-consistency theorem, global liveness claim, automatic stale-state repair, or production readiness result follows from the inspected local mechanics.

## Sources

- [Production operator runbooks](../../production-operator-runbooks.md)
- [Receipt-first cluster harness](../../receipt-first-cluster-harness.md)
- [Daemon startup, status, and shutdown implementation](../../../src/node/parts/daemon/p018/body.rs)
- [Lifecycle classification and restart implementation](../../../src/node/parts/daemon/p036/body.rs)
- [Shutdown preservation regression cases](../../../src/node/parts/daemon/tests/m000/p014/body.rs)
- [Technical companion](../README.md)
