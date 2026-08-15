# Change: harness-generic-replay-gate

## Why

Generic deterministic replay receipts now exist and vat fixtures consume them, but harness pass gates still treat deterministic replay as a check string plus an ad hoc replay block. Gate receipts should bind `deterministic-replay-verify-v1` as a first-class evidence artifact so reports, repro bundles, and future subsystems can reuse the same replay verification shape.

## What

- Add generic replay verification evidence to harness gate checks and gate receipts.
- Bind the generic replay receipt ref in gate artifact refs.
- Validate embedded generic replay receipts when parsing gate receipts.
- Preserve existing harness replay comparisons and checks.

## Impact

Harness pass evidence gains a reusable deterministic replay receipt boundary without changing authority semantics. A passing generic replay receipt remains evidence only and does not grant policy, capability, resource, transport, provenance, or source-gate trust.
