## Context

Receipt stage vectors are ordered. During `full`, cleanup can append a successful `stop` stage even after an earlier stage failed. A last-stage-only status is misleading for acceptance history.

## Decisions

### 1. Status precedence

**Choice:** List status uses deterministic precedence: `failed` if any stage failed; otherwise `running`, then `pending`, then `skipped`, then `succeeded` when all recorded stages succeeded, else `empty`.

**Rationale:** Failed acceptance should dominate cleanup success. Running/pending are retained for partially written receipts. Skipped is distinct from success.

**Alternative:** Add a top-level receipt status field. Rejected for this slice because existing receipts can be summarized correctly without schema migration.

## Risks / Trade-offs

- **Mixed historical receipts:** Existing receipts benefit immediately because status is computed from stages.
- **Future partial receipt semantics:** If later stages become explicitly skipped, the precedence already surfaces `skipped` rather than success.
