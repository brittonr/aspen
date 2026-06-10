## Context

`local-artifact-catalog` introduced short-id resolution for UI/CLI convenience. `canonical-content-ref-discipline` tightened full content refs, but short-id resolution still needs a crisp input grammar so ref-shaped malformed values do not silently become prefix searches.

## Goals

- Treat canonical full refs and short hex prefixes as different input classes.
- Keep short ids lowercase hex-only and scheme-free.
- Deny malformed `blake3:` values before visible candidate enumeration.
- Keep ambiguity and hidden-ref filtering unchanged for valid short prefixes.
- Preserve canonical receipt/result records for denial decisions.

## Non-Goals

- No new hash algorithm or alternate ref scheme.
- No change to catalog artifact identity or registry storage.
- No remote catalog or transport behavior.
- No acceptance of uppercase prefixes; this slice chooses explicit rejection over normalization.

## Input grammar

Catalog short-id resolution accepts exactly two forms:

1. A full canonical content ref accepted by `preserves_rail::validate_content_ref`.
2. A nonempty lowercase hex prefix (`[0-9a-f]+`) with no `blake3:` prefix.

Values with the canonical content-ref prefix that fail validation are malformed full refs, not short-id prefixes. Values with uppercase letters, punctuation, or other characters are malformed short prefixes.

## Denial behavior

Malformed inputs produce canonical short-id denial results with diagnostics such as:

- `malformed full content ref: ...`
- `short id prefix must use lowercase hex characters`
- `short id prefix requires at least N hex characters`
- `short id prefix matched no visible refs`
- `short id prefix is ambiguous across N visible refs`

Candidate search is skipped for malformed input and for prefixes shorter than the requested minimum.

## MCP behavior

`catalog.short_id` and `short_id_resolve` route through the same catalog core. The MCP response remains a canonical deny response rather than a transport failure for malformed provided prefixes. Missing arguments remain argument-validation denials.

## Tests

- Direct catalog tests cover malformed `blake3:` refs, uppercase prefix rejection, hidden-only matches, full canonical ref lookup, ambiguity, and minimum length.
- MCP tests cover malformed ref-shaped prefixes returning deny responses with diagnostics.
