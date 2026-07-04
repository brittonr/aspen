# Design: Contract invariant negative fixtures

## Context

Contracts become trustworthy when their valid and invalid boundaries are executable. Production profile fixtures already model this well; the same discipline should cover plugin extensions, grants, peer profiles, multinode scenarios, and Cairn policy contracts.

## Fixture taxonomy

Use one negative fixture per invariant where practical:

- malformed BLAKE3 ref
- empty required evidence array
- unsupported enum or vocabulary typo
- duplicate id or descriptor
- missing schema metadata
- stale or mismatched schema metadata
- unsafe path or invalid stable id
- inverted numeric/window relationship
- missing proof, authority, policy, resource, or effect evidence
- stale internal cross-reference

Positive fixtures should cover the reviewed happy path and any edge value intentionally allowed by the contract.

## Validation output

Where practical, fixture filenames and validation diagnostics should name the expected failure class. This makes accidental failures due to parse errors, import mistakes, or unrelated predicates visible.

## Boundary

Fixtures prove authoring contract behavior only. They do not replace runtime receipts, source-gate freshness, adapter conformance, or production drills.
