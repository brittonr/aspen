# Design: replay receipt catalog

## Ledger classification

The ledger `artifact_kind` table recognizes:

- `deterministic-replay-verify-v1` as `deterministic-replay-verify-receipt`;
- `deterministic-first-divergence-v1` as `deterministic-first-divergence`.

## Catalog classification

Catalog semantic classification recognizes both the compact harness replay verification shape and the larger fixture replay verification shape. Classifications include:

- `deterministic-replay:verify`;
- `replay-decision:<decision>`;
- `receipt-decision:<decision>`;
- `replay-divergence:<kind>`;
- expected/actual report refs for harness gate replay receipts;
- expected/actual identity, effect-log, output, and final-state refs for fixture replay receipts.

First-divergence records classify:

- `deterministic-replay:first-divergence`;
- `replay-divergence:<kind>`;
- actor id, handler profile ref, expected ref, and actual ref.

## Boundaries

Catalog classification only makes replay evidence discoverable. It does not validate the original run, authorize any operation, replace harness gate parsing, or grant trust in policy/capability/resource/provenance/source gates.
