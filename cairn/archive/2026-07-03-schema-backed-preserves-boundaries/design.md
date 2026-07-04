# Design: schema-backed Preserves boundaries

## Scope

This change introduces `preserves-schema` validation at selected external Preserves record boundaries. It does not replace semantic validation; schemas prove record shape and field classes, while existing gates prove authority, provenance, policy, resource, and replay semantics.

## Proof checklist

- **Proof claim**: selected external Preserves record families must pass versioned schema validation before semantic admission.
- **Out of scope**: generating every Molten DTO from schema in one slice.
- **Trusted assumptions**: checked-in schema artifacts and their expected refs are reviewed as part of release evidence.
- **Positive evidence**: canonical valid fixtures pass schema and existing semantic validation.
- **Negative evidence**: wrong label, missing schema id, missing field, wrong type, malformed checks, and unsupported version deny before side effects.
- **Canonical refs**: schema artifact ref, value ref, boundary family, schema version, and validation receipt/diagnostic ref.
- **Regeneration command**: `cargo test schema plugin node evidence retention dogfood`.

## Functional core

Add a pure schema validation adapter that accepts an `IOValue`, an expected schema ref/name, and a compiled schema description. Imperative shells load checked-in schema artifacts, call the pure validator, and then pass validated values to existing parsers.

## Phasing

Begin with a narrow allowlist of high-risk boundary families. After validation is stable, later changes may promote more record families or generate typed Rust parsers from schema definitions.

## Non-goals

- No runtime Nickel execution for schema loading.
- No weakening of existing semantic gates when schema validation passes.
