# Proposal: Replay Evidence Index

## Summary

Add deterministic replay index evidence that groups replay verification receipts and replay rollups into a reusable readback artifact for fixture suites, catalogs, and MCP inspection.

## Problem

Replay verification receipts and rollups are individually searchable, but larger suites need a stable evidence artifact that can be imported, read back, and filtered without every consumer rescanning raw replay receipts. The index must remain evidence-only and must not grant authority, policy admission, source-gate acceptance, transport trust, or release trust.

## Goals

- Emit `deterministic-replay-index-v1` evidence over bounded replay verify receipts and replay rollups.
- Validate expected refs against canonical hashes and deny stale, tampered, or unsupported inputs.
- Summarize total, pass, deny, raw receipt, rollup, divergence, report, and final-state evidence.
- Make replay indexes importable through the ledger and searchable through catalog/MCP replay evidence UX.

## Non-Goals

- Granting authority or replacing replay verification/gate validation.
- Trusting labels or catalog metadata as replay proof.
- Making replay evidence a source-gate, release-gate, policy, provenance, transport, or resource admission token.
