# Diagnosing restart and lock inconsistency

Mode: Troubleshooting

A node restart error is a reason to identify the last observed boundary, not permission to clear a lock. Preserve the selected root, non-secret lifecycle artifacts, and collection context before attempting further effects. This guide is source-checked and was not executed for this document. The [operator runbooks](../../production-operator-runbooks.md) remain the operational review authority; the [lifecycle companion](../../technical/operations/node-lifecycle-and-recovery.md) supplies background rather than an automatic repair procedure.

## First distinguish observation from mutation

`node status` is not a neutral diagnostic: it writes health and control receipts and imports them. Prefer inspecting existing captured artifacts before creating a new observation. `node show` reads a supplied Preserves artifact and prints its supported summary; the summary is not canonical verification or a process probe.

For a reviewed non-secret artifact, this command's spelling and behavior are supported by the [Show declaration](../../../src/cli/ops/node/command/base.rs), [main alias](../../../src/main.rs), and [show shell](../../../src/cli/ops/node/parts/lifecycle/p001/body.rs). Source-checked; not executed.

```sh
: "${LIFECYCLE_ARTIFACT:?Set a preserved non-secret lifecycle artifact path}"
molten node show "$LIFECYCLE_ARTIFACT"
```

Record configuration, identity receipt, startup, shutdown, and active-lock presence separately. Also record permission or read failures: an inaccessible root is not evidence of an empty root. The path-based compatibility inspector returns `Empty` if opening fails, whereas the root-bearing inspection API exposes errors. Do not use that convenience classification as an incident inventory by itself.

## Symptom: restart lacks a clean shutdown receipt

**Discriminating evidence:** restart requires clean completion for the exact current run and no active lock. Startup and shutdown filenames alone are insufficient: missing, unreadable or conflicting run/completion records reject restart. Separate runs may have identical startup bytes, so compare the run binding and its previous-run link as well as startup content.

**Safe next action:** identify the previous startup and retain any control, adapter, and shutdown-attempt evidence. Determine whether shutdown admission was denied, never attempted, or interrupted during effects. A human recollection that the process ended cannot substitute for the missing boundary evidence.

**Stop condition:** if effect completion is unknown, stop before another run or stop attempt. Do not delete the lock, synthesize shutdown receipts, or initialize over the root. There is no automatic arbitrary-crash recovery recipe established by this path.

## Symptom: dispatch requires an active node lock

**Discriminating evidence:** the dispatcher calls `require_active_lock` before selecting a pending request. Missing `control/node.lock.preserves` is distinct from an empty inbox. A stopped node normally has shutdown evidence and no active lock; an initialized node has neither startup nor shutdown. A denied startup can already have published a startup receipt before lock installation.

**Safe next action:** classify the complete lifecycle set and inspect the startup decision. Match the intended root against the root used to collect evidence. If this is a genuinely new local exercise, choose a separate fresh root rather than repurposing the incident root.

**Stop condition:** do not create a lock by hand or dispatch against another root merely because it contains a similarly named request. Capability-bound entries are scoped to their originating root and namespace.

## Symptom: lock is stale for the current startup

**Discriminating evidence:** `require_active_lock` compares both startup and run refs against the current canonical run binding. An old run cannot borrow a new run's identity even when their startup refs match. A readable stale lock is distinct from a missing lock, malformed artifact or observation read error.

**Safe next action:** retain both artifacts and identify whether they came from different attempts, an incomplete restore, or concurrent writes. These are hypotheses to investigate, not diagnoses proved by the message. Compare canonical content and provenance rather than modifying the lock to match the latest filename.

**Stop condition:** suspend dispatch while startup binding remains unresolved. Renaming host directories is not a reliable retargeting mechanism for a process that already holds an open `DirectoryRoot`.

## Symptom: stop failed, but some shutdown artifacts exist

**Discriminating evidence:** admission denial and effect failure have different consequences. Denial returns before executing a shutdown plan and writes control-denial evidence. An admitted plan can publish adapter, node shutdown, control and exact run-completion evidence before removing the lock. An I/O error can follow earlier completed writes. A later denied repeat stop does not invalidate successful completion: status/restart resolve the immutable control receipt named by that completion, not the mutable latest stop-control file.

**Safe next action:** inspect the decision and referenced results in `stop-control-receipt.preserves`; inventory individual adapter and node shutdown artifacts. The [shutdown regression cases](../../../src/node/parts/daemon/tests/m000/p014/body.rs) distinguish preservation on denial from effect errors and reverse adapter shutdown order.

**Stop condition:** do not interpret an unsuccessful command as proof that nothing happened. Do not retry uncertain effects unconditionally or delete partial evidence to make the directory look consistent.

## Worked case: old shutdown success while the current lock remains

Shutdown request R completes in run A, then the node restarts as run B. Exact R remains a duplicate with the original result; it does not execute again or stop B's loop. A consistent B remains active even if A and B have identical startup content. Stopping B requires a distinct request and fresh admission, not an unconditional retry of R.

Preserve both run contexts and their original receipts. If an old completion is placed beside B's active lock, current observation rejects the conflicting evidence without new shutdown effects or lock mutation. Conversely, a service can correctly report current completion while denying because other queued work exceeded its bound. Inspect its decision and stopped observation separately. None of these labels establishes process liveness, durable completion of every external effect, or safe replay; runbook backup, retention, source-candidate and recovery authority remain separate.

## Sources

- [Handbook](../README.md)
- [Production operator runbooks](../../production-operator-runbooks.md)
- [Lifecycle companion](../../technical/operations/node-lifecycle-and-recovery.md)
- [Restart and classification](../../../src/node/parts/daemon/p036/body.rs)
- [Lock validation](../../../src/node/parts/daemon/p034/body.rs)
- [Status and shutdown ordering](../../../src/node/parts/daemon/p018/body.rs)
- [Shutdown regression cases](../../../src/node/parts/daemon/tests/m000/p014/body.rs)
- [Current-run observations](../../../src/node/parts/daemon/p058/body.rs)
- [Run-bound regression cases](../../../src/node/parts/daemon/tests/m000/p023/body.rs)
