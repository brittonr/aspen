# Design: replay-evidence-rollup

## Overview

`deterministic-replay-rollup-v1` summarizes a bounded set of generic replay verification receipts. The rollup validates each input value against its expected content ref when provided, parses only `deterministic-replay-verify-v1` records, and records diagnostics for stale or invalid inputs.

## Shape

The rollup records:

- schema marker `molten.determinism.replay-rollup.v1`
- decision: `pass` only when every input is readable and every replay verify receipt passed
- total, pass, and deny counts
- sorted receipt refs
- divergence counts by kind
- sorted first-divergence refs when present
- diagnostics
- evidence-only checks

## Catalog/MCP

Ledger import classifies the schema as `deterministic-replay-rollup`. Catalog classifications include:

- `deterministic-replay:rollup`
- `replay-rollup-decision:<decision>`
- `replay-rollup-total:<n>`
- `replay-rollup-pass:<n>`
- `replay-rollup-deny:<n>`
- `replay-rollup-divergence:<kind>`

The existing `search_replay_evidence` MCP tool discovers rollups via `stage=rollup` plus normal text filters.

## Evidence boundary

A rollup is a readback/index artifact. It does not grant authority, admit policy, prove source-gate acceptance, prove provenance, or replace verification of individual replay receipts.
