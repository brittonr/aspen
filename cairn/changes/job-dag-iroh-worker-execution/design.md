## Context

The intended path is staged: local execution, remote sync, target admission, loopback execution, then live remote worker execution. This change assumes `job-dag-loopback-execution` provides canonical execution request/receipt verification and reuses that contract over remote dataspace/Iroh.

## Goals

- Define `job-worker-request-v1`, `job-worker-assignment-v1`, `job-worker-status-v1`, `job-worker-result-v1`, and `job-worker-receipt-v1`.
- Carry worker requests/results through `remote-dataspace-envelope-v1` with protocol/session refs when available.
- Require target-side sync/admission/execution evidence before running stages.
- Bind node identity, peer bootstrap agreement, authority context, resource profile, selected stages, target storage/cache/chunk profile, and replay log refs.
- Support deterministic loopback/local-gossip worker tests and live Iroh diagnostics.
- Deny arbitrary source paths, mobile closures, unverified artifacts, missing admission, stale sync, and mismatched target peer.

## Non-Goals

- No global scheduler.
- No source-node execution after target assignment.
- No implicit worker authority from peer identity.
- No unrecorded live network timing as deterministic pass evidence.
- No support for non-deterministic stages as pass evidence.

## Records

```preserves
<job-worker-request-v1 "molten.job-dag.worker-request.v1"
  <job <job-ref>>
  <target-peer "peer:b">
  <stages ["stage-id" ...]>
  <sync <job-sync-receipt-ref>>
  <admission <job-admission-receipt-ref>>
  <execution-request <job-execution-request-ref>>
  <authority [<authority-context-ref> ...]>
  <resource [<resource-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <checks [<check "target-admission-required" "pass"> ...]>>
```

```preserves
<job-worker-result-v1 "molten.job-dag.worker-result.v1"
  <decision "pass"|"deny"|"non-replayable">
  <job <job-ref>>
  <target-peer "peer:b">
  <execution-receipt <job-execution-receipt-ref>>
  <outputs [<output-ref> ...]>
  <stage-receipts [<stage "id" <receipt-ref>> ...]>
  <resource [<resource-receipt-ref> ...]>
  <delivery-log <remote-dataspace-delivery-log-ref>>
  <diagnostics ["..." ...]>
  <checks [<check "executed-on-target-state" "pass"> ...]>>
```

## Worker Flow

1. Source computes sync plan and target missing set.
2. Target fetches/verifies closure and emits sync receipt.
3. Target emits admission receipt with authority/resource checks.
4. Source or control plane submits worker request referencing sync/admission/execution request evidence.
5. Target verifies request, replays loopback execution verifier, executes selected stages from target roots only, and emits result.
6. Delivery logs and result receipts are imported into the evidence ledger/catalog.

## Replay

Recorded local-gossip delivery logs are replayable. Live Iroh worker messages are diagnostics until all worker request/status/result bytes and effect logs are captured; gate receipts must distinguish replayable and non-replayable worker runs.
