# Change: Node Control Live Send Retry and Idempotency UX

## Motivation

External live node-control sends currently bind a receiver ticket and transport receipt, but operators need a deterministic way to pre-bind the derived operation id, retry bounded join/publish failures, and recognize duplicate sends without re-broadcasting. The UX must keep transport evidence separate from authority/provenance and fail closed with canonical receipts.

## Proposed Change

Add live-send retry/idempotency evidence for node control:

- `control-ingress-live-send` accepts an optional `--operation-id` guard and bounded `--max-attempts`.
- Failed live transport join/publish attempts emit canonical retry receipts instead of unreceipted process errors.
- Re-sending an already successful state-root-bound live send emits a duplicate-send receipt and reuses the prior send receipt without another transport publish.
- Send diagnostics identify operation-id mismatch, join timeout/failure, unsupported ticket addresses, and duplicate suppression.

## Non-Goals

- Live transport retries do not create authority, peer bootstrap, policy/resource, provenance, or delivery-idempotency evidence.
- This change does not introduce a global sequence counter or mutable transport session registry.
- This change does not make live Iroh runs deterministic replay evidence; replay remains receipt-bound and transport observations remain non-authority.
