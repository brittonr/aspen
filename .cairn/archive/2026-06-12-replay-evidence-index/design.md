# Design: Replay Evidence Index

## Overview

`deterministic-replay-index-v1` is a deterministic evidence receipt built from replay verification receipts and replay rollups. It validates each supplied value against its expected content ref, parses only known replay evidence schemas, and emits a deny index with diagnostics when any input is stale, tampered, or unsupported.

## Receipt shape

The index includes:

- schema marker `molten.determinism.replay-index.v1`
- decision (`pass` or `deny`)
- total replay evidence count
- pass and deny counts
- raw replay verification receipt count
- replay rollup count
- indexed replay verify receipt refs
- indexed replay rollup refs
- divergence counts by kind
- first-divergence refs
- report refs when present
- final-state refs when present
- diagnostics
- evidence-only checks

## Validation

The index recomputes canonical hashes for every input. If an expected ref is supplied, the index validates the ref shape and denies when it does not match the input value. Unsupported values are not counted as replay evidence and produce diagnostics.

Rollup inputs are treated as summarized replay evidence: their total/pass/deny/divergence counts and embedded receipt refs are included in the index. Raw verify receipts are parsed directly and contribute their refs, decisions, divergence kind, and report/final-state refs.

## Evidence-only boundary

Replay indexes are reusable readback evidence only. They do not replace the individual replay verify receipts, rollup receipts, harness gates, policy gates, source gates, provenance checks, transport checks, or release promotion checks. Consumers must continue to validate their own gates and referenced evidence.

## UX

The CLI adds `molten test replay-fixture index --receipt ... --rollup ... --out ...`. Catalog import classifies indexes as `deterministic-replay-index`, and replay MCP search can discover them with `stage=index`.
