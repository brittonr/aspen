# Design: preserves rail parser toolkit

## Scope

This change centralizes common Preserves helper code without changing canonical record layouts. It covers builders and parsers for simple records, schema fields, required strings, content refs, optional refs, ref sequences, check sets, bounded lists, and diagnostics.

## Proof checklist

- **Proof claim**: migrated modules parse and build the same canonical values through shared helper functions.
- **Out of scope**: wholesale DTO/codegen conversion for every record family.
- **Trusted assumptions**: existing canonical record labels and field order remain normative until separate schema migrations change them.
- **Positive evidence**: before/after helper migrations produce identical canonical hashes for representative fixtures.
- **Negative evidence**: malformed records, wrong arity, wrong type, invalid ref, missing check, and duplicate/unsupported check cases fail closed.
- **Canonical refs**: fixture value refs before and after migration, helper error class, and module boundary name.
- **Regeneration command**: `cargo test preserves_rail service job schema protocol node retention catalog plugin evidence`.

## Functional core

All toolkit functions are pure over `IOValue`, borrowed field views, or immutable inputs. Call-site shells remain responsible for file/database/network reads and for converting helper errors into receipts or CLI diagnostics.

## Migration strategy

Adopt the toolkit in small module groups with hash-stability tests. Preserve compatibility wrappers where public modules currently expose helper-shaped APIs.

## Non-goals

- No receipt schema churn in this change.
- No runtime allocation-heavy reflection framework beyond bounded helper traversal.
