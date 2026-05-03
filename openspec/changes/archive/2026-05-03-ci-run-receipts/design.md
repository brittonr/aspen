## Context

`PipelineRun` is already persisted in Aspen KV under `_ci:runs:<run-id>` and powers `ci status`. It has enough stage/job/deploy status detail to create a stable operator receipt without adding another write path.

## Decisions

### 1. Receipt projection from persisted run

**Choice:** Generate `aspen.ci.run-receipt.v1` from `PipelineRun` when queried.
**Rationale:** This is the smallest operator-visible slice, avoids duplicate persistence consistency bugs, and keeps evidence cluster-backed because the source run already lives in Raft KV.
**Alternative:** Persist a second receipt key when runs complete; deferred until operators need immutable snapshots independent of run-schema evolution.

### 2. Append-only RPC surface

**Choice:** Add `CiGetRunReceipt { run_id }` and `CiGetRunReceiptResult`.
**Rationale:** Native CLI and future dogfood integrations need typed access without scraping `ci status` output. Aspen RPC enums are compatibility contracts, so variants are appended rather than inserted in domain order.

### 3. JSON-clean CLI output

**Choice:** Add `aspen-cli ci receipt <run-id> [--json]`; human output is concise, JSON emits the structured receipt.
**Rationale:** Operators need both transcript-friendly summaries and machine-checkable evidence.

## Risks / Trade-offs

- A projected receipt reflects the current run record shape, not an immutable historical receipt snapshot. This is acceptable for the first slice and documented.
- `PipelineRun.stages[*].jobs` is a map, so receipt generation sorts stage jobs by name for deterministic output.
