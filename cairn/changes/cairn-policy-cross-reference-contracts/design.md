# Design: Cairn policy cross-reference contracts

## Context

The current policy contract is strong at field boundaries but weaker at whole-policy relationships. The generated policy should only contain relationships that can be resolved against declared source entries.

## Reference indexes

Build pure helper predicates over the in-memory Nickel record:

- collect artifact ids from all artifact schemas and verify every `requires` item exists.
- collect marker ids and tokens and require both sets to be distinct.
- collect replay case ids and replay group ids and verify determinism surfaces reference known entries.
- collect receipt schema commands and verify receipt contracts bind known command/schema surfaces.

Each helper returns a boolean contract predicate. No helper reads files, runs commands, or consults generated JSON.

## Fixture strategy

Add one positive policy fixture with all relationship classes present. Add negative fixtures for unknown artifact dependency, duplicate marker token/id, stale replay case, stale replay group, and duplicate or unknown receipt command.

## Boundary

This change strengthens source validation only. It does not remove Rust validation, generated policy checks, Cairn gates, or release evidence review.
