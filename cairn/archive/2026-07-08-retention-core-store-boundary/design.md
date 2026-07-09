## Context

Retention logic protects destructive operations. It also owns many evidence artifacts and store layouts. A modular split must preserve fail-closed behavior and make it obvious that discovery, planning, applying, executing, auditing, exporting, and live clearance have different trust boundaries.

## Design

### Proposed retention modules

- `model`: retention classes, actions, evidence summaries, object refs.
- `admission`: pure destructive admission and denial diagnostics.
- `plan`: pure GC candidate and plan construction.
- `apply`: pure plan drift and apply-gate decisions.
- `audit`: pure chain consistency checks.
- `store`: retention evidence store port and filesystem adapter.
- `bundle`: explain/bundle export and verification shell.
- `live`: remote-clearance request/response/import workflow shell.
- `receipts`: canonical retention receipt constructors and parsers.

### Destructive-operation law

No shell should delete, tombstone, redact, compact, unpin, or import remote clearance as authoritative unless a pure retention decision returns an admitted plan for that exact object/action/class scope.

### Test strategy

Use in-memory evidence summaries for pure admission tests and shell-level fixtures for store/bundle/live behavior. Negative tests must cover missing authority, stale plan, drifted plan, incomplete reference index, missing remote clearance, and overbroad evidence.

## Non-goals

- Do not relax retention admission.
- Do not make bundle verification authority.
- Do not make remote live transport proof of deletion clearance.
