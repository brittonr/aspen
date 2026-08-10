# Design: bound inherited Tracey debt

## Goal

Make the inherited uncovered set explicit and fail closed on traceability regression.

## Search results

Three mechanisms were evaluated.

1. Add markers for every accepted requirement.
   This route was rejected because a generated marker does not prove an implementation or verification link.
2. Use only the pinned Cairn report.
   This route was rejected because its hard-coded scan omits most Molten source roots.
3. Scan all admitted evidence roots and bind the exact uncovered set.
   This route preserves missing coverage as debt and detects growth without false closure.

## Design

A Rust tool separates pure marker classification from filesystem collection.
The tool reads accepted specifications from `cairn/specs/`.
It reads evidence markers from `src/`, `crates/`, `tests/`, `tools/`, `docs/`, `scripts/`, and `flake.nix`.

The tool compares the sorted uncovered set with a checked-in baseline.
It denies these conditions:

- a new uncovered requirement;
- a removed baseline entry without a reviewed baseline update;
- a dangling evidence marker;
- a malformed baseline;
- an unsorted or duplicate baseline.

A typed Nickel record binds the baseline count, BLAKE3 digest, roots, and non-claims.
The generated JSON copy remains deterministic.

## Boundaries

The baseline is debt evidence, not an exemption.
A requirement remains uncovered until direct source or verification evidence references it.
The guard does not prove marker truth, implementation correctness, test adequacy, release readiness, or whole-system correctness.
The pinned Cairn command remains the lifecycle authority for this legacy layout.
