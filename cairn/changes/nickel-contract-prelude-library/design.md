# Design: Nickel contract prelude library

## Context

Multiple Nickel modules independently define the same predicates. A small shared prelude can reduce duplication while preserving local module ownership for domain-specific invariants.

## Module shape

Introduce a source-controlled prelude module, for example `docs/nickel-contract-prelude.ncl`, that exports pure predicates and contracts:

- `is_non_empty_string` / `NonEmptyString`
- `is_non_empty_array` / `NonEmptyArray`
- `is_blake3_ref` / `Blake3Ref`
- `is_stable_id` / `StableId`
- `is_positive_integer` / `PositiveInteger`
- `is_one_of` / allowed-value contract constructor
- `has_all`, `distinct_strings`, and `array_of`
- exact metadata predicate constructors for schema id, version, and source language

Domain modules continue to define their own vocabulary lists, cross-field invariants, envelope shape, and runtime evidence caveats.

## Migration order

Migrate one module at a time, preserving exported fixture values. Start with modules that already use equivalent helper definitions: production profiles, multinode scenarios, peer profiles, plugin extension contracts, and Cairn policy contracts.

## Boundary

The prelude is not a runtime dependency for Rust admission. Runtime code continues to consume checked generated artifacts.
