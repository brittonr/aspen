# Design: Plugin extension domain contract hardening

## Context

Runtime parsing already validates plugin extension Preserves records with concrete domain checks. The Nickel authoring layer should reject the same obvious invalid shapes before export so reviewers do not need to wait for Rust admission to catch authoring mistakes.

## Functional core

Keep predicate logic pure and deterministic:

- `is_blake3_ref`: exact lowercase `blake3:` content refs.
- `is_plugin_extension_id`: non-empty ids with the `plugin-extension:` prefix and reviewed character set.
- `is_extension_version`: semver-compatible reviewed text without accepting arbitrary prose.
- `is_profile`: exact `production` or `development`.
- `is_non_empty_ref_array`: non-empty arrays where every entry is a BLAKE3 ref.
- `descriptors_are_unique`: no repeated `(operation, descriptor_ref)` pair.
- `attenuation_is_coherent`: current depth is at most max depth and validity turns are ordered.

The pure predicates should be small enough to test with positive and negative fixtures. The imperative shell remains the existing export/regeneration path.

## Runtime boundary

Rust remains authoritative for checked-in Preserves evidence. Nickel does not become runtime authority, does not read live state, and does not prove plugin behavior. It only gates human-maintained source fixtures before canonical exports are refreshed.

## Compatibility

Existing valid plugin extension and grant fixtures should continue to export. Any checked-in invalid fixture must either remain explicitly negative or be corrected before export drift gates are enabled.
