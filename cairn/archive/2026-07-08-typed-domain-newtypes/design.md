# Design: typed domain newtypes

## Scope

This change narrows Rust in-memory representations for common evidence domains. It does not change external Preserves layouts, JSON exports, or CLI argument syntax unless a later migration explicitly does so.

## Proof checklist

- **Proof claim**: invalid refs, decisions, statuses, and domain identifiers cannot enter migrated pure cores without passing typed constructors.
- **Out of scope**: converting every repository struct in one slice.
- **Trusted assumptions**: constructors encode the same reviewed domain predicates used by existing parsers.
- **Positive evidence**: migrated DTOs roundtrip to the same canonical Preserves bytes.
- **Negative evidence**: invalid content refs, unsupported decisions, empty stable ids, and unsupported replay classes fail at construction or parse time.
- **Canonical refs**: before/after fixture refs for migrated record families.
- **Regeneration command**: focused parser/newtype tests plus representative CLI fixture checks.

## Functional core

Define pure domain types with `parse`, `as_str`, and conversion methods. Builders accept typed domains where possible. Parsers convert external values to typed domains before returning DTOs.

## Imperative shell

CLI and filesystem shells continue to read strings from user inputs and files, then parse them at the boundary. Error plumbing reports the specific domain that failed.

## Migration

Migrate high-risk DTOs first: content refs, decisions, check statuses, schema ids, plugin operations, replay classes, and consensus profile ids. Defer broad mechanical cleanup until fixtures prove hash stability.
