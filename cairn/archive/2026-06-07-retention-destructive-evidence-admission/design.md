## Overview

Add a typed retention admission receipt layer between explicit destructive evidence refs and retention eligibility evaluation. The prior slice required callers to supply refs; this slice makes those refs meaningful by requiring local Preserves admission values whose canonical refs match the supplied refs and whose scope matches the candidate destructive operation.

## Design

### Admission receipts

Introduce `retention-evidence-admission-v1` values with:

- `kind`: `policy`, `authority`, `supporting-evidence`, `reference-index`, or `remote-gc`
- `decision`: `pass` or `deny`
- `requester`, `object`, `class`, and `action` scope
- bound refs for the underlying policy/authority/evidence/index/remote facts
- retained and remote refs relevant to the proof
- revocation refs and diagnostics
- checks proving canonical binding, scope binding, and non-grant separation

Admission receipts are stored under the local retention store and addressed by canonical ref. Supplied destructive evidence refs must resolve to these values.

### Destructive admission

For each destructive candidate, ledger GC, chunk GC, and eval-cache invalidation call a pure validation helper that:

1. validates syntax and required explicit inputs;
2. reads and parses policy, authority, supporting-evidence, reference-index, and remote-GC admission receipts;
3. requires matching requester, object ref, object kind, class, and action;
4. rejects deny/stale/revoked/mismatched receipts;
5. requires a reference-index admission when a destructive apply claims the reference index is complete;
6. denies retained refs and unresolved remote refs unless matching remote-GC admission clears the remote refs.

The returned admission decision drives `has_delete_authority` and `has_remote_gc_clearance` in the lower-level retention evaluation.

### Receipts and CLI

Destructive subsystem receipts include admission diagnostics and admitted refs. The CLI accepts reference-index and remote-GC evidence refs in addition to the existing evidence flags.

## Risks

- Existing tests using synthetic refs need fixture helpers that write admission receipts first.
- Dry-run commands should remain useful for planning; they may report missing admission diagnostics without mutating state.
