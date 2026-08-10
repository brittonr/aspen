# Design: Repair runtime-spine Tracey links

## Selection rule

A requirement leaves the inherited baseline only when review identifies:

- a specific production function or type that implements the accepted behavior;
- a focused positive or negative test that exercises the same behavior;
- no broader wording that exceeds the observed implementation or test boundary.

The reviewed Preserves rail cluster satisfies this rule for fourteen requirements.
Broader requirements, including trust-boundary-wide adoption and semantic admission ordering, remain uncovered.

## Evidence manifest

A typed Nickel manifest records each requirement identifier, source area, implementation path, verification path, and evidence scope.
Generated JSON is a deterministic tool input only.

## Freshness gate

The inherited Tracey Nix check exports the manifest, verifies its generated JSON, and checks each listed marker at its declared path.
It also rejects any reviewed identifier that remains in the exact inherited baseline.
The existing classifier must reproduce the grouped TSV and Markdown reports byte-for-byte.

## Functional boundary

This change adds comments and evidence metadata only.
The existing pure Preserves rail core and focused tests remain unchanged.

## Non-claims

The repair proves direct source linkage for the listed requirements only.
It does not prove all runtime-spine behavior, all Preserves boundaries, release readiness, or whole-system correctness.
