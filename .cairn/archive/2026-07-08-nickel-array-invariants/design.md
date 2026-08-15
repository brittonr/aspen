# Design: Nickel array invariants

## Scope

This change tightens repository-owned Nickel authoring contracts. It does not alter runtime Rust admission except through reviewed checked-in exports and existing Preserves evidence gates.

## Proof checklist

- **Proof claim**: reviewed arrays whose semantics require uniqueness, bounds, or required membership reject ambiguous or oversized fixtures at authoring time.
- **Out of scope**: making all arrays globally sorted or unique when duplicates are meaningful.
- **Trusted assumptions**: contract authors identify per-field uniqueness and bound semantics.
- **Positive evidence**: current valid fixtures still export after helper adoption.
- **Negative evidence**: duplicate peer refs, duplicate adapters, duplicate lifecycle callbacks, duplicate refs, oversized arrays, and missing required members fail export.
- **Canonical refs**: generated JSON refs for affected drift-gated exports.
- **Regeneration command**: contract export drift gate and production profile fixture checks.

## Functional core

Add pure Nickel helper contracts for arrays: bounded length, non-empty bounded length, uniqueness by identity, unique BLAKE3 refs, and required-member inclusion. Compose these helpers with existing element contracts instead of local ad hoc predicates.

## Imperative shell

The flake validation exports fixtures and compares generated JSON. Runtime code continues to consume checked exports and Preserves evidence only.

## Migration

Apply helpers incrementally to arrays with clear semantics. If a duplicate array is intentionally meaningful, document the exception near the contract field.
