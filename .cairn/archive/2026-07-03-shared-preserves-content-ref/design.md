# Design: shared Preserves content ref

## Scope

This change unifies canonical BLAKE3 content refs used by Preserves-boundary records. It covers the shared newtype, serde/string compatibility, and staged migration of parsed DTOs that currently use raw strings for canonical refs.

## Proof checklist

- **Proof claim**: internally represented content refs are canonical by construction after parsing.
- **Out of scope**: changing the public `blake3:<hex>` wire format or switching hash algorithms.
- **Trusted assumptions**: BLAKE3 remains Molten's content-addressing algorithm for Preserves canonical bytes.
- **Positive evidence**: valid lowercase BLAKE3 refs parse, serialize, compare, and render identically to existing strings.
- **Negative evidence**: wrong prefix, wrong length, uppercase hex, non-hex characters, empty string, and path-like strings fail to parse.
- **Canonical refs**: parsed ref string, typed DTO field, conversion call site, and boundary parser name.
- **Regeneration command**: `cargo test preserves_rail runtime artifacts typed job eval catalog schema`.

## Functional core

Keep the content-ref type as a pure validated value object with no I/O. Boundary parsers convert strings into the newtype. Builders and CLI shells convert back to strings only when rendering Preserves values, JSON, or user-facing output.

## Migration strategy

Start with runtime envelope `ContentRef` and DTOs that already validate refs. Then migrate additional parsed structs in small slices with compatibility constructors where needed.

## Non-goals

- No public CLI syntax changes.
- No acceptance of additional hash prefixes without a separate compatibility and migration change.
