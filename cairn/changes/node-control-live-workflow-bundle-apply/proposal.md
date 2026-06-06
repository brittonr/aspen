# Change: Node Control Live Workflow Bundle Apply UX

## Motivation

Live workflow bundles can now be exported, verified, gated, and imported, but operators still have to manually stitch the final sender-side workflow together. That leaves room to import stale bundles, skip the gate they reviewed, or accidentally send over live Iroh before doing a dry-run preflight.

## Proposed Change

Add `molten node live-workflow-bundle-apply` and canonical `node-control-live-workflow-bundle-apply-receipt-v1` receipts. The command revalidates the bundle and optional gate receipt, imports bundle members only after validation, dry-runs live-send preflight by default when a request is supplied, and performs the live send only with explicit `--send`.

## Non-Goals

- Apply receipts do not replace authority grants, peer admissions, policy/resource refs, provenance refs, delivery-idempotency evidence, or receiver-side ingress receipts.
- Apply does not make transport identity authoritative.
- Apply does not weaken the standalone import or live-send fail-closed checks; those checks are repeated during orchestration.
