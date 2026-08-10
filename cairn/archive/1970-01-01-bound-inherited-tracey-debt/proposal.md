# Proposal: bound inherited Tracey debt

## Problem

The pinned legacy Cairn checker reports 2,468 accepted requirements and only 218 referenced requirements.
Its hard-coded evidence scan reads `crates/` and `tools/` but omits `src/`, `tests/`, documentation, and `flake.nix`.
A comprehensive scan finds additional valid references, but 1,980 requirements still have no source marker.
Blanket marker generation would create false coverage claims.

## Change

- Repair verified Artifact adoption marker placement and source references.
- Add a deterministic repository-owned guard for the comprehensive evidence roots.
- Store the inherited uncovered set as a sorted, BLAKE3-bound baseline.
- Deny new missing requirements, new dangling references, or unreviewed baseline changes.
- Keep every uncovered requirement visible without claiming behavioral correctness.

## Scope

This change manages inherited traceability debt.
It does not claim repository-wide Tracey closure or implementation correctness for uncovered requirements.
It does not change runtime behavior, authority, transport, release policy, or dependency pins.
