# Proposal: Release Replay Evidence Binding

## Summary

Bind deterministic replay evidence indexes into operator dogfood and release evidence as mandatory evidence-only readback.

## Problem

Replay verification, rollup, and index receipts are reusable evidence, but release/dogfood evidence did not yet require a replay index ref. Release readback should detect missing, stale, or tampered replay index evidence without treating it as authority, policy, source-gate, provenance, transport, or release trust.

## Goals

- Make local dogfood emit a replay evidence index and bind its ref into the release gate.
- Make Nix dogfood evidence, release bundles, and bundle verification read back replay index refs.
- Deny stale, missing, or tampered replay index files during release evidence readback.
- Classify release artifacts that bind replay indexes for catalog/MCP discovery.
- Preserve evidence-only semantics.

## Non-Goals

- Granting release authority from replay evidence.
- Replacing source gates, policy gates, provenance checks, Octet checks, Cairn validation, signed keyring checks, or harness replay verification.
