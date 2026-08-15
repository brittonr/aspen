# Catalog Delta: Canonical Short ID Prefixes

### Requirement: Canonical short-id prefix grammar
r[molten.catalog.short_id_canonical_prefixes] Molten MUST treat catalog short-id inputs as either canonical full content refs or lowercase hex prefixes without a `blake3:` scheme, and MUST NOT treat malformed ref-shaped strings as prefix searches.

#### Scenario: Ref-shaped malformed prefix denies
- GIVEN a catalog short-id input of `blake3:` or `blake3:<bad>`
- WHEN short-id resolution runs
- THEN the decision is `deny`
- AND candidate search is skipped with a malformed full-ref diagnostic

#### Scenario: Full canonical ref resolves exactly
- GIVEN a full canonical content ref visible in the catalog
- WHEN short-id resolution receives that full ref
- THEN the decision is `pass`
- AND the result expands to the same full ref

### Requirement: Short-id malformed denials
r[molten.catalog.short_id_malformed_denials] Molten MUST deny non-hex or uppercase short-id prefixes as canonical data-bearing denial results before downstream catalog operations receive them.

#### Scenario: Uppercase prefix denies
- GIVEN a short-id prefix containing uppercase hex characters
- WHEN short-id resolution runs
- THEN the decision is `deny`
- AND diagnostics state that short-id prefixes use lowercase hex characters

#### Scenario: Hidden-only prefix denies
- GIVEN a lowercase hex prefix that matches only hidden refs
- WHEN short-id resolution runs with those refs hidden
- THEN the decision is `deny`
- AND no hidden full ref is returned as the resolution
