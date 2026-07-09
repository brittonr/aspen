## Context

Accepted project requirements already say shared Nickel helpers are authoring-time only. This change generalizes that boundary for Cairn policy and runtime policy consumption. Aspen's checked-in generated Cairn policy currently fails against the current sibling Cairn CLI because the generated JSON lacks a newer `traceability_policy` field, so freshness needs to be explicit.

## Design

### Policy layers

- `authoring`: Nickel/Cairn policy source, contracts, fixtures, and helper libraries.
- `export`: deterministic generated JSON or Preserves artifacts checked into source control.
- `runtime consumption`: Rust admission code reads typed checked exports or canonical refs.
- `freshness`: validation proves generated artifacts match reviewed source and current schema.

### Runtime boundary

Runtime admission may consume policy refs, checked policy exports, or policy-gate receipts. It must not execute Nickel, run Cairn policy export, or use policy tooling availability as live authority.

### Freshness validation

A focused validation should fail when generated policy is stale relative to reviewed source or current schema. Diagnostics should identify missing fields, duplicate ids, stale refs, or malformed exports.

### Tests

Positive fixtures cover valid authoring source and fresh generated exports. Negative fixtures cover stale generated JSON, missing schema fields, duplicate ids, bad refs, and runtime code attempting live policy-tool execution.

## Non-goals

- Do not remove Nickel authoring.
- Do not introduce runtime Nickel evaluation.
- Do not make Cairn CLI availability part of runtime trust.
