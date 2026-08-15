# Design: Production profile schema metadata

## Context

Production profile exports are evidence artifacts. Evidence artifacts need explicit identity so receipts can state which schema they validated and which source boundary produced them.

## Metadata fields

Add root metadata fields such as:

- `schema` for the canonical production profile schema id.
- `schema_version` for compatibility and migration decisions.
- `source_language` with the reviewed value `nickel`.
- `profile_id` or equivalent stable profile identity separate from display name when needed.

The exact exported names should follow the existing JSON naming conventions used by runtime config and production receipts.

## Receipt binding

Deployment-profile and startup receipts should bind the profile metadata alongside the profile content ref. Validation should reject missing, unsupported, or mismatched schema/source metadata before accepting a profile as production evidence.

## Evidence-only boundary

Metadata establishes identity and review boundary only. It must not be treated as authority, provenance, source-gate pass, adapter conformance, resource sufficiency, or live transport readiness.
