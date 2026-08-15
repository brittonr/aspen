## Context

Current no-disabled evidence shows `non_trait_imports` as the dominant warning family. Import cleanup is broad and mechanical, so it benefits from a narrow package that can accept repeated small reductions without requiring architectural decisions for unrelated lint families.

## Design

### Import-hygiene boundary

This change owns source edits whose primary evidence is a lower `non_trait_imports` count. Acceptable slices include:

- removing broad or redundant imports;
- qualifying names at use sites when it lowers import debt without adding path-repetition debt;
- moving private helper types near their callers to avoid import fan-out;
- replacing import-heavy shell glue with smaller module-local dispatch helpers.

Each slice must preserve public Rust module names, CLI paths, receipt labels, canonical Preserves values, and fail-closed denial behavior.

### Validation

Each accepted slice should run the smallest relevant Rust check, then a no-disabled Octet probe. Documentation must record at least the before/after count for `non_trait_imports`, the probe output path, and any unchanged caveats.

### Non-goals

- Do not relax Octet policy.
- Do not change public command syntax or receipt schemas.
- Do not claim source-remediated zero while other disabled families or source-scope blockers remain.
