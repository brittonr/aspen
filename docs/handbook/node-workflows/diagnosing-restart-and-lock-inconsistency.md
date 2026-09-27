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

**Discriminating evidence:** `verify_restart_state` first checks for startup. If startup exists and shutdown does not, it returns the diagnostic that the previous startup has no clean shutdown receipt. The active lock is not the deciding fact at this branch. The lifecycle regression deliberately reaches this denial by running again after restart without stopping.

**Safe next action:** identify the previous startup and retain any control, adapter, and shutdown-attempt evidence. Determine whether shutdown admission was denied, never attempted, or interrupted during effects. A human recollection that the process ended cannot substitute for the missing boundary evidence.

**Stop condition:** if effect completion is unknown, stop before another run or stop attempt. Do not delete the lock, synthesize shutdown receipts, or initialize over the root. There is no automatic arbitrary-crash recovery recipe established by this path.

## Symptom: dispatch requires an active node lock

**Discriminating evidence:** the dispatcher calls `require_active_lock` before selecting a pending request. Missing `control/node.lock.preserves` is distinct from an empty inbox. A stopped node normally has shutdown evidence and no active lock; an initialized node has neither startup nor shutdown. A denied startup can already have published a startup receipt before lock installation.

**Safe next action:** classify the complete lifecycle set and inspect the startup decision. Match the intended root against the root used to collect evidence. If this is a genuinely new local exercise, choose a separate fresh root rather than repurposing the incident root.

**Stop condition:** do not create a lock by hand or dispatch against another root merely because it contains a similarly named request. Capability-bound entries are scoped to their originating root and namespace.

## Symptom: lock is stale for the current startup

**Discriminating evidence:** `require_active_lock` parses the lock schema and compares its `startup` field to the canonical ref of the current startup receipt. A readable lock with an old binding is not equivalent to a missing lock or malformed artifact.

**Safe next action:** retain both artifacts and identify whether they came from different attempts, an incomplete restore, or concurrent writes. These are hypotheses to investigate, not diagnoses proved by the message. Compare canonical content and provenance rather than modifying the lock to match the latest filename.

**Stop condition:** suspend dispatch while startup binding remains unresolved. Renaming host directories is not a reliable retargeting mechanism for a process that already holds an open `NodeStateRoot`.

## Symptom: stop failed, but some shutdown artifacts exist

**Discriminating evidence:** admission denial and effect failure have different consequences. Denial returns before executing a plan and writes control-denial evidence. An admitted plan writes adapter receipts, node shutdown, and control receipts, imports them, then removes the lock. An I/O error in that sequence can follow earlier completed writes.

**Safe next action:** inspect the decision and referenced results in `stop-control-receipt.preserves`; inventory individual adapter and node shutdown artifacts. The [shutdown regression cases](../../../src/node/parts/daemon/tests/m000/p014/body.rs) distinguish preservation on denial from effect errors and reverse adapter shutdown order.

**Stop condition:** do not interpret an unsuccessful command as proof that nothing happened. Do not retry uncertain effects unconditionally or delete partial evidence to make the directory look consistent.

## Worked case: status says stopped while the lock remains

The status path reports `stopped` when it finds shutdown evidence; it does not use the same five-fact classification as the lifecycle inspector. Configuration, identity, startup, shutdown, and lock together are therefore an inconsistent classifier combination even if status's projection says stopped.

Preserve both observations. Check where shutdown publication ended relative to lock removal, and whether the evidence was captured concurrently. This is a source-review explanation of possible divergence, not a reproduced bug. Neither label establishes process liveness, durable completion of every external effect, or safe replay. A passing restart-health decision likewise does not replace backup, retention, source-candidate, and authorized recovery evidence required by the runbooks.

## Sources

- [Handbook](../README.md)
- [Production operator runbooks](../../production-operator-runbooks.md)
- [Lifecycle companion](../../technical/operations/node-lifecycle-and-recovery.md)
- [Restart and classification](../../../src/node/parts/daemon/p036/body.rs)
- [Lock validation](../../../src/node/parts/daemon/p034/body.rs)
- [Status and shutdown ordering](../../../src/node/parts/daemon/p018/body.rs)
- [Shutdown regression cases](../../../src/node/parts/daemon/tests/m000/p014/body.rs)
