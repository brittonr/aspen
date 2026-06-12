# Change: vat-generic-replay-receipts

## Why

The deterministic replay fixture now emits generic `deterministic-replay-verify-v1` and `deterministic-first-divergence-v1` evidence, while the vat replay fixture still reports replay through vat-local receipts. The next draining slice should connect the vat evidence surface to the generic replay records so future replay consumers do not need vat-specific logic for first-divergence and pass/deny boundaries.

## What

- Bind the vat replay fixture to generic deterministic replay verification receipts.
- Include at least one passing generic replay verification receipt and one first-divergence denial in vat replay evidence.
- Keep vat-local receipts as compatibility and scenario evidence while making generic receipts available for downstream gates.
- Document that generic replay receipts remain evidence-only and do not grant vat authority, transport trust, policy admission, or source-gate trust.

## Impact

Vat replay becomes the first consumer of the generic replay spine, proving that subsystem fixtures can reuse the runtime-level replay evidence shape before transcripts, evaluation cache, remote sync, and job DAG replay adopt it.
