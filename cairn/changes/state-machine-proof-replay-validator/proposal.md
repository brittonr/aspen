## Why

Matrix and property tests prove local decision laws, but release review also needs proof-carrying traces: a reviewer should be able to replay a sequence of state-machine receipts and see where every transition, decision, diagnostic, and state ref came from. A trace validator closes the loop between local proof tests and durable evidence.

## What Changes

- Add requirements for replay-validating state-machine proof traces.
- Require fail-closed negative validation for missing, stale, tampered, or out-of-order proof evidence.
- Define the minimal evidence fields a proof trace must bind.

## Impact

- **Files**: testing harness, replay/evidence validators, and proof fixtures.
- **Testing**: positive trace replay, tampered receipt denial, stale state-ref denial, missing diagnostic denial, and deterministic summary rendering.
