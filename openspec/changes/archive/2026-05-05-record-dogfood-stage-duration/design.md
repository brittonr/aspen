## Context

`aspen-dogfood` writes schema-versioned JSON run receipts during `full`. Receipts include ordered stages, status, timestamps, failures, and artifacts. The operator can list/show/diagnose local receipts and show receipts published into cluster KV. Stage duration is currently inferable only by parsing timestamp strings.

## Goals / Non-Goals

**Goals:**
- Add explicit stage elapsed milliseconds to newly written receipts.
- Keep pre-existing receipts valid when `elapsed_ms` is absent.
- Show elapsed timing in human-readable receipt inspection output.
- Keep receipt JSON deterministic and bounded.

**Non-Goals:**
- Change the schema constant or require migration of existing receipt files.
- Run a fresh end-to-end dogfood cluster in this slice.
- Add cross-run trend storage or dashboards.

## Decisions

### 1. Optional compatible JSON field

**Choice:** Add `elapsed_ms: Option<u64>` to `DogfoodStageReceipt` with serde default and skip-when-none behavior.

**Rationale:** This makes new receipts richer while preserving operator access to already-saved local and cluster receipts.

**Alternative:** Bump `aspen.dogfood.run-receipt.v1` to a v2 schema. Rejected because the change is additive and backward-compatible.

**Implementation:** Record elapsed duration from a monotonic `Instant` when a stage starts and serialize it only after a stage reaches a terminal status.

### 2. Human-readable output includes duration

**Choice:** `receipts show` prints `elapsed_ms=<value>` on each stage line, with `-` when absent.

**Rationale:** Operators should see slow stages directly without rerendering JSON or manually diffing timestamps.

## Risks / Trade-offs

**Timestamp/duration mismatch** → Mitigated by treating timestamps as wall-clock evidence and elapsed milliseconds as monotonic process-local evidence; both are useful but neither is used as a security proof.

**Legacy receipts missing duration** → Mitigated by `Option<u64>` and display fallback.
