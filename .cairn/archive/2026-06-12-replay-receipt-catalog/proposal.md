# Change: replay-receipt-catalog

## Why

Generic deterministic replay receipts are now emitted by fixtures and harness gates, but ledger/catalog classification does not expose them as first-class searchable evidence. Operators and dogfood release workflows need to find replay verification and first-divergence records directly by decision, divergence kind, report refs, and final-state refs instead of grepping gate receipts.

## What

- Classify `deterministic-replay-verify-v1` and `deterministic-first-divergence-v1` in the evidence ledger.
- Add catalog classifications for replay decision, divergence kind, expected/actual report refs, output/state refs, and first-divergence refs.
- Add tests proving imported replay receipts are searchable through catalog filters.

## Impact

Replay receipts become reusable catalog evidence for operator workflows and future release evidence bundles. Classification remains evidence-only and does not grant authority, policy admission, replay trust beyond the receipt content, provenance trust, or source-gate trust.
