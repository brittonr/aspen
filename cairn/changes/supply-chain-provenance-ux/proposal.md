## Why

Molten has canonical provenance records and node-control provenance gates, but operators need a direct CLI surface to materialize provenance records, run the same trust-state evaluation used by node-control, and inspect provenance receipts while debugging install/run denials.

## What Changes

- Add `molten test provenance` commands for synthetic reviewed fixtures, explicit provenance records, provenance evaluation receipts, and artifact summaries.
- Reuse the existing provenance record and receipt DTOs so CLI diagnostics match node-control install/run gates.
- Keep provenance UX evidence-only: provenance receipts explain trust-state admission, but do not grant authority, policy, resource, transport, execution, or source-gate trust.
- Add CLI coverage for reviewed pass and sandbox-only node-control denial.

## Impact

Operators can reproduce provenance decisions without running a node-control dispatch path, and future supply-chain slices can build on a stable provenance diagnostics command group.
