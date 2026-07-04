# Change: contract-export-drift-gates

## Why

Reviewed Nickel source, checked-in generated JSON or Preserves exports, Preserves boundary schemas, and Rust admission parsers can drift independently. A contract may be tightened without regenerating exports, or an export may remain parseable while no longer matching the authoring source.

## What

- Add deterministic drift gates that compare Nickel source exports against checked-in JSON/Preserves artifacts.
- Verify Preserves boundary schema identity/arity and Rust parser admission for valid exports and rejection for negative exports.
- Run the gate without network, production credentials, mutable runtime state, or runtime Nickel authority.

## Impact

Evidence refreshes fail before stale generated artifacts are promoted. Contract, export, boundary-schema, and Rust-admission behavior remain aligned for release review.
