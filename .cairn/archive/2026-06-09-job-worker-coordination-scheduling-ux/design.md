## Context

`job-dag-iroh-worker-cli-ux` exposed direct worker request construction and recorded local-gossip execution. The next operator gap is scheduling: worker requests need queue admission and a claim/lease proof before execution, with stale fencing tokens denied before side effects.

## Goals

- Route scheduled worker execution through the existing coordination runtime instead of ad hoc in-memory job selection.
- Use the coordination queue for FIFO worker-request admission.
- Apply the enqueue request twice to prove duplicate operation-id replay does not enqueue twice.
- Use a coordination lock acquisition as the worker lease and bind the emitted fencing token into job schedule evidence.
- Deny stale token overrides before worker execution and emit a schedule receipt with the coordination denial evidence.
- Write a durable evidence directory containing schedule receipt, coordination manifest/report/evidence, queue receipts, lease token, release receipt, and nested worker execution evidence.
- Import schedule receipts to the ledger when a ledger root is supplied and show them in `job status`/`receipt-show`.

## Non-Goals

- No persistent distributed scheduler daemon.
- No global cross-process queue state beyond the recorded local command run.
- No replacement for authority, policy, resource, source-gate, provenance, sync, target admission, or execution request checks.
- No claim that queue membership, fencing token possession, transport delivery, or CLI invocation grants execution authority.

## Implementation Notes

The CLI command is an imperative shell around existing pure/canonical primitives:

1. Parse the canonical `job-worker-request-v1` artifact.
2. Build a fixture coordination runtime and apply queue enqueue/dequeue plus lock acquire/release requests with explicit refs.
3. Before worker execution, compare any provided lease token override with the acquired fencing token; mismatches apply a release attempt that produces a stale-token denial and skip worker execution.
4. Run the existing `worker-run-local` helper only after lease validation passes.
5. Build `job-worker-schedule-receipt-v1` as evidence binding coordination and worker receipts.

The command is intentionally replay-friendly: all scheduling and worker artifacts are written under the output directory, and the coordination apply report binds final state and operation receipts.
