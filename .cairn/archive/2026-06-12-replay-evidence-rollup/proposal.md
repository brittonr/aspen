# Change: replay-evidence-rollup

## Why

Replay verify receipts are now reusable evidence and searchable through catalog/MCP, but release and operator workflows still need a single evidence artifact that summarizes a set of replay checks. Without a rollup, operators must gather and count individual replay verify receipts manually.

## What

- Add `deterministic-replay-rollup-v1` as an evidence-only summary over replay verification receipts.
- Count total, pass, deny, and divergence kinds while preserving refs to individual receipts and first-divergence evidence.
- Deny rollups with stale/mismatched receipt refs or unreadable replay verification inputs.
- Classify replay rollups in ledger/catalog and expose them through existing replay MCP search.

## Impact

Rollups make suite/release replay evidence easier to inspect and import. They remain explanatory evidence only and do not replace individual replay verification, harness gates, source gates, provenance, authority, or policy admission.
