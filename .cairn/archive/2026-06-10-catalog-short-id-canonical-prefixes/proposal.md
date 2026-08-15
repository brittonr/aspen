## Why

Catalog short-id resolution still accepted ref-shaped prefixes such as `blake3:` as ordinary search prefixes. After canonical content-ref validation, that can blur the boundary between full content refs and UI-only short ids.

Molten should make the catalog boundary explicit: full refs must be canonical content refs, while short ids are lowercase hex prefixes only and are never identity.

## What Changes

- Require catalog short-id inputs to be either a full canonical content ref or a lowercase hex prefix without the `blake3:` scheme.
- Deny malformed full refs such as `blake3:` or `blake3:<bad>` before candidate search.
- Deny uppercase or non-hex short prefixes with specific diagnostics.
- Preserve hidden-ref filtering, ambiguity denial, and full-ref expansion semantics.
- Add catalog core and MCP regressions for malformed refs, uppercase prefixes, hidden-only matches, and canonical full-ref lookups.

## Impact

Catalog CLI/MCP users get clearer diagnostics and fewer accidental matches. Canonical records continue to store full refs only, while short ids remain display conveniences that must expand before downstream operations.
