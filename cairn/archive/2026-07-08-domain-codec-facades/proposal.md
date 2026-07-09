## Why

`preserves_rail` is a central canonical boundary, but direct use across many domains makes it a global coupling point. Domain code that constructs raw Preserves records everywhere is harder to review, harder to extract, and easier to subtly diverge from the intended schema semantics.

## What Changes

- Introduce domain-owned codec façades for receipt, manifest, envelope, and admission values.
- Keep canonical Preserves encoding and BLAKE3 identity centralized, but stop requiring most domains to assemble raw records directly.
- Add positive and negative tests for façade constructors and parsers.
- Preserve canonical byte identity for existing artifacts unless a separate schema change owns a versioned break.

## Impact

This change reduces coupling to `preserves_rail` while preserving the canonical Preserves spine. Domain modules become easier to move into crates because they depend on narrow codec interfaces rather than a broad global helper module.
