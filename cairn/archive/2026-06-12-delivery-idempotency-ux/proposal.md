## Why

Delivery idempotency already guards remote ingress and several runtime paths, but operators need a direct CLI surface to inspect canonical scope refs, operation ids, first/duplicate/conflict/gap receipts, and stored dedup receipts while debugging replay or live node-control workflows.

## What Changes

- Add `molten test delivery` commands for scope profile refs, operation id materialization, idempotency checks, stored receipt lookup, and artifact summaries.
- Keep the CLI evidence-only: it emits canonical Preserves records and receipts but does not grant transport, authority, provenance, or policy trust.
- Bind CLI checks to the same Redb-backed delivery idempotency store used by remote dataspace/node-control ingress paths.
- Add coverage for first delivery, duplicate suppression, prior receipt binding, and stored receipt display.

## Impact

Operators can now reproduce and inspect dedup/replay decisions without running a full live transport loop, and future roadmap slices can reference a stable delivery-idempotency UX for diagnostics.
