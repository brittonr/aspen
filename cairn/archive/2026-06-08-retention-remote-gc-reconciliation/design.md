## Context

`retention-destructive-evidence-admission` introduced typed admission receipts and rejects forged, stale, revoked, or mismatched evidence refs. Remote refs are still represented as caller-supplied refs plus `remote-gc` admissions. That proves a local admission exists, but it does not prove each remote/peer was separately reconciled or that a remote still does not retain the object.

## Decisions

### 1. Model per-remote clearance as evidence-only receipts

**Choice:** Introduce `retention-remote-gc-clearance-v1` values stored under the retention state root. Each receipt binds decision, requester, peer ref, object ref/kind, retention class, action, remote ref, policy ref, authority ref, supporting evidence refs, retained refs, freshness, revocation refs, diagnostics, and checks.

**Rationale:** Peer-specific clearance is auditable and can be supplied independently from local policy/authority admissions. It remains deletion-safety evidence only and does not grant authority, policy, resource, transport, provenance, execution, or source-gate trust.

### 2. Reconcile all declared remote refs before mutation

**Choice:** Extend destructive evidence with `remote_clearance_refs`. Admission passes remote clearance only when every declared remote ref has a current passing clearance with no retained refs and a matching scope/policy/authority binding. Missing, stale, revoked, mismatched, retained, or forged clearances deny before mutation.

**Rationale:** Local destructive operations must fail closed when the reference index or configured remote set is incomplete or when a remote remains uncertain.

### 3. Keep remote-GC admissions and remote clearance distinct

**Choice:** Existing `remote-gc` admissions still admit the local remote-GC plan. New clearance receipts prove per-remote reconciliation and are required in addition to remote-GC admissions when remote refs are present.

**Rationale:** A local plan is not the same as peer-specific clearance. Keeping both refs separate avoids treating support evidence as authority or remote proof.

## Risks / Trade-offs

- Operators must provide more evidence for destructive cleanup involving remote refs.
- Current implementation is local receipt admission; future live remote protocols can produce the same canonical clearance values.
