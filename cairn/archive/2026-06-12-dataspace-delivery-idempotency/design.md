## Context

Iroh gossip and many distributed transports are at-least-once, reordered, delayed, or replay-prone. Molten's semantics must treat delivery evidence separately from semantic commitment. A canonical idempotency layer lets remote dataspace, protocol sessions, service lifecycle, and job workers reject duplicates or resume safely.

## Goals

- Define `operation-id-v1`, `delivery-window-v1`, `dedup-entry-v1`, `delivery-idempotency-receipt-v1`, and `retry-receipt-v1`.
- Add idempotency scope profiles for actor turn, service lifecycle, protocol session, remote dataspace topic, job worker request, and control-plane command.
- Compute operation refs from canonical sender, receiver, scope, sequence, payload ref, and intent class.
- Check dedup/sequence windows before committing local dataspace side effects.
- Distinguish idempotent duplicate, conflicting duplicate, stale sequence, future gap, retry, and replay-denied outcomes.
- Persist dedup windows through the ledger/Redb adapter with retention/GC pins.

## Non-Goals

- No exactly-once network transport claim.
- No global sequence across all actor messages.
- No dedup identity based on wall-clock timestamps or local paths.
- No dedup bypass for privileged operators.

## Records

```preserves
<operation-id-v1 "molten.delivery.operation-id.v1"
  <scope <scope-ref>>
  <producer <authority-or-actor-ref>>
  <consumer <actor-or-service-ref>>
  <sequence 42>
  <intent "dataspace-assert"|"message"|"job-worker"|"protocol-send">
  <payload <payload-ref>>
  <policy [<policy-ref> ...]>
  <checks [<check "canonical-operation-ref" "pass"> ...]>>
```

```preserves
<delivery-idempotency-receipt-v1 "molten.delivery.idempotency-receipt.v1"
  <decision "first"|"duplicate"|"conflict"|"stale"|"gap"|"retry"|"deny">
  <operation <operation-ref>>
  <scope <scope-ref>>
  <window <delivery-window-ref>>
  <prior <prior-receipt-ref-or-none>>
  <side-effect "commit"|"suppress">
  <diagnostics ["..." ...]>
  <checks [<check "dedup-before-commit" "pass"> ...]>>
```

## Runtime Algorithm

1. Derive operation ref from canonical intent before admission/commit.
2. Load dedup window for scope.
3. If unseen and sequence is valid, record first-delivery receipt and allow admission to proceed.
4. If seen with identical payload/evidence, emit duplicate receipt and suppress repeated side effects while returning the prior semantic result ref.
5. If seen with a different payload/evidence, emit conflict denial.
6. If stale or too far ahead, emit stale/gap denial or retry receipt according to delivery class.

## Retention

Dedup windows are retained by explicit scope policies. GC cannot remove window entries required by active protocol sessions, job assignments, service lifecycle, or replay logs.
