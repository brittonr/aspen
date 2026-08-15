## Phase 1: Spec and parser

- [x] [serial] r[molten.catalog.short_id_canonical_prefixes] Specify canonical full-ref vs short-prefix grammar for catalog short-id resolution.
- [x] [serial] r[molten.catalog.short_id_malformed_denials] Implement fail-closed malformed full-ref and malformed hex-prefix denials before candidate search.

## Phase 2: Catalog and MCP tests

- [x] [serial] r[molten.catalog.short_id_malformed_denials] Add catalog regressions for malformed `blake3:` refs, uppercase prefixes, hidden-only matches, and canonical full-ref lookup.
- [x] [serial] r[molten.catalog.short_id_malformed_denials] Add MCP regressions proving malformed short-id inputs return canonical deny responses with diagnostics.

## Phase 3: Validation

- [x] [serial] r[molten.catalog.short_id_canonical_prefixes] Run fmt, tests, clippy, Octet, and Cairn gates before archive.
