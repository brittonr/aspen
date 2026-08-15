## Context

The canonical Preserves boundary is a core Molten law. The modularity issue is not the boundary itself; it is that many domains call broad helper functions directly and construct low-level records inline. That scatters schema knowledge and makes `preserves_rail` a high-fan-in dependency.

## Design

### Domain façade pattern

Each domain that emits canonical artifacts should own narrow functions or types such as:

- `receipt_value(input)` for canonical receipt construction;
- `parse_receipt(value)` for typed parse and validation;
- `manifest_value(input)` for manifest construction;
- `canonical_ref(input)` or `artifact_ref(value)` where identity is part of the domain contract.

These façades call the shared codec layer internally. Callers should use the domain façade rather than assembling raw record labels and field sequences directly.

### Canonical identity preservation

The first migration must be byte-preserving for existing schemas. Tests should compare canonical refs or rendered canonical text for representative fixtures before and after façade introduction. Any non-preserving change must be modeled as an explicit schema/version change outside this package.

### Parser and constructor symmetry

Façades should pair constructors with parsers when the domain later consumes the same artifact kind. Negative parser tests should cover wrong schema labels, missing required fields, malformed content refs, duplicate fields where disallowed, and unsupported versions.

### Dependency direction

Domain modules may depend on the narrow codec interface or their own façade. The broad codec module should not import high-level domains.

## Non-goals

- Do not replace Preserves as the canonical boundary.
- Do not change BLAKE3 content identity.
- Do not weaken runtime parser validation in favor of authoring-time contracts.
