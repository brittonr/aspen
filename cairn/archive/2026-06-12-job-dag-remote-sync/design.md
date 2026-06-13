## Context

`local-job-dag` and `local-job-dag-planning` define local execution and planning. Distributed job execution needs a prior synchronization step: stage artifacts and their dependency closure must be present and verified at the target before a remote peer can consider admission.

This slice is loopback/local only. It deliberately avoids real network transport and remote execution while shaping the canonical records that later Iroh sync can carry.

## Goals

- Define transport-neutral job sync request, plan, and receipt DTOs.
- Compute closure roots from the job DAG artifact plus selected stage artifact refs.
- Compute target missing sets from full artifact refs, not names or paths.
- Install missing artifacts into a target local registry in dependency-first order.
- Verify installed artifact refs and canonical artifact envelopes after import.
- Preserve fail-closed no-execution behavior.

## Non-Goals

- No remote process execution.
- No arbitrary closure shipping.
- No network transport yet.
- No trust in mutable names, paths, mtimes, or target display metadata.
- No policy bypass: sync only makes artifacts available.

## Records

```preserves
<job-sync-request-v1 "molten.job-dag.sync-request.v1"
  <job <job-ref>>
  <stages ["stage-id" ...]>
  <target-peer "peer:loopback">
  <policy [<policy-ref> ...]>
  <capability [<capability-ref> ...]>
  <evidence [<evidence-ref> ...]>
  <checks [<check "no-execution" "pass"> ...]>>
```

```preserves
<job-sync-plan-v1 "molten.job-dag.sync-plan.v1"
  <request <request-ref>>
  <job <job-ref>>
  <target-peer "...">
  <roots [<artifact-ref> ...]>
  <closure [<artifact-ref> ...]>
  <missing [<artifact-ref> ...]>
  <stages ["stage-id" ...]>
  <checks [<check "dependency-closure" "pass"> ...]>>
```

```preserves
<job-sync-receipt-v1 "molten.job-dag.sync-receipt.v1" ...>
```

Plan receipts bind the sync plan ref. Loopback receipts bind installed refs, already-present refs, closure refs, and checks.

## Algorithm

1. Resolve the job DAG from the source registry by job ref or job artifact ref.
2. Find the source job artifact ref.
3. Add selected stage artifact refs; if no stage ids are selected, include all stage artifact refs.
4. Compute the source dependency closure.
5. Compare closure refs against the target registry.
6. For loopback sync, install missing refs in dependency-first DFS order.
7. Verify each target artifact envelope matches the source artifact envelope and requested ref.
8. Emit receipt. Do not execute stages.

## Tests

- Empty target receives job artifact and stage dependency closure.
- Repeated sync reports already-present/no-op behavior.
- Tampered or missing dependency closure rejects before install.
- Sync receipts include no-execution and hash verification checks.
