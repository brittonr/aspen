# Design: Node Control Live Send Retry and Idempotency UX

## Receipt model

The existing `node-control-live-send-receipt-v1` remains the canonical final send receipt. This change adds two auxiliary receipt classes:

- `node-control-live-send-retry-receipt-v1` records a bounded failed attempt, the attempt index, max attempts, receiver ticket, envelope, derived operation ref, diagnostics, and checks.
- `node-control-live-send-duplicate-receipt-v1` records a duplicate suppression event, binding the derived operation ref, receiver ticket, envelope, and prior pass send receipt.

Auxiliary receipts are imported into the node ledger when a state root is supplied. They are evidence of operator/transport behavior only; they never satisfy receiver-side bootstrap, authority, policy/resource, provenance, or enqueue gates.

## Operation-id guard

The live send envelope already derives a scoped delivery operation id from topic, destination node, producer peer, sequence, payload request ref, and policy refs. The CLI exposes this as an `--operation-id` guard: if supplied, the command derives the envelope first and fails closed with a deny send receipt when the derived operation ref differs.

## Retry behavior

`--max-attempts` is bounded. Each failed live Iroh join/publish attempt produces a retry receipt. After all attempts fail, the command emits the final deny send receipt with accumulated diagnostics. A successful later attempt emits a pass send receipt while preserving retry receipts as auxiliary evidence.

## Duplicate behavior

For state-root-bound sends, Molten recomputes the deterministic pass transport/send receipt refs for the derived envelope before opening a live transport. If the prior pass send receipt is present and matches the envelope, Molten emits a duplicate-send receipt and returns the prior send receipt, suppressing another broadcast.

## Failure diagnostics

Malformed tickets, missing addresses, unsupported address forms, operation-id mismatches, join timeouts, join failures, publish failures, and stale duplicate receipt paths fail closed with diagnostics. Diagnostics are canonical receipt fields and are suitable for runbook/workflow review.
