## Context

`job-dag-iroh-worker-execution` introduced canonical worker request/result/receipt records and a recorded local-gossip execution path. The remaining gap is ergonomic: users can run sync, admission, and loopback execution from the CLI, but cannot directly assemble a worker request or run the worker transport/evidence path.

## Goals

- Generate `job-worker-request-v1` from a target admission receipt and execution request artifact.
- Default request evidence to include sync, admission, execution request, peer bootstrap, and node identity refs when supplied.
- Run a deterministic local-gossip worker harness that uses `job_worker_envelope`, `publish_local_gossip`, `deliver_local_gossip`, `delivery_log`, and `execute_worker_delivery`.
- Write a durable output directory with request, envelope, transport receipts, delivery log, assignment, status records, worker result, worker receipt, execution receipt, and output when present.
- Import worker artifacts into the ledger when requested.
- Summarize worker receipts with `job receipt-show` and include worker receipts in `job status`.

## Non-Goals

- No global scheduler or daemon.
- No new authority model; worker requests still require target admission, authority receipts, resource refs, peer bootstrap, node identity, and evidence refs.
- No claim that live network timing is deterministic pass evidence.
- No source-registry execution after target assignment.

## Implementation Notes

The new CLI is an imperative shell around existing worker core functions. `worker-request` parses existing admission and execution request artifacts, validates their binding, derives the canonical worker request, and emits Preserves. `worker-run-local` records a replayable local-gossip delivery log before executing the request on target registry/storage/cache/chunk roots. Denials remain represented by worker result/receipt diagnostics.
