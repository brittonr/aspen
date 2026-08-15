## Why

Retention-gated destructive paths now evaluate retention before removal, but the GC and invalidation callers still synthesize requester, policy, evidence, and authority refs internally. Apply-mode deletion must be driven by explicit operator/policy/authority evidence and complete reference-index inputs, not local placeholder refs.

## What Changes

- Add explicit retention gate evidence inputs for ledger GC, chunk-store GC, and evaluation-cache invalidation.
- Require apply-mode destructive operations to deny when requester, policy, authority, evidence, or reference-index proof inputs are missing.
- Thread retained refs, remote refs, and reference-index completeness into per-candidate retention receipts.
- Expose retention evidence flags and diagnostics on the CLI while preserving dry-run as diagnostic planning evidence.
- Add fail-closed tests for missing authority/policy/evidence, incomplete indexes, retained refs, and remote uncertainty.

## Impact

Destructive maintenance remains auditable and becomes policy/evidence driven. Retention receipts stay deletion-safety evidence only and do not grant authority, provenance, policy, resource, transport, execution, or source-gate trust.
