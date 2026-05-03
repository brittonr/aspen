## Context

`receipts show` exposes full receipt detail, and docs map stages/categories to operator checks. The new command should not add a second evidence format; it should derive a diagnosis from the same validated `DogfoodRunReceipt` used by list/show.

## Decisions

### 1. Diagnose is a read-only receipt command

**Choice:** Add `receipts diagnose <run-id-or-path>` beside `list` and `show`.

**Rationale:** Operators already know to start with `receipts`; keeping diagnose under that namespace makes it an evidence interpretation command rather than a live cluster command.

**Alternative:** Add flags to `show`. Rejected because diagnosis is a different operator task from displaying the receipt.

### 2. Use deterministic built-in triage text

**Choice:** Map the first failed stage and failure category to static recommended checks.

**Rationale:** The output is stable, testable, and safe to run offline. It can be refined later without changing receipt schema.

**Alternative:** Inspect logs or call live cluster APIs. Rejected because the command should work after cleanup and should not require credentials.

## Risks / Trade-offs

- **Advice can become stale** → Keep recommendations short and grounded in current runbook categories.
- **Successful receipts need useful output** → Print that no failed stage exists and direct operators to `show` for archival details.
- **Credential exposure** → Never read ticket files; only print receipt paths and generic warnings.
