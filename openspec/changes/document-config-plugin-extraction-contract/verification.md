# Verification

## Evidence

- Inventory source: `crates/aspen-nickel/Cargo.toml` shows default dependencies on `nickel-lang`, `aspen-core`, serde/serde_json, SNAFU/thiserror, and tracing.
- Inventory source: `crates/aspen-plugin-api/Cargo.toml` shows a leaf default graph over `serde`, `serde_json`, and `semver` with no Aspen workspace dependencies.
- Changed file: `docs/crate-extraction/config-plugin.md` documents owner, feature contract, dependency decisions, representative consumers, exceptions, and verification rails.
- Changed file: `docs/crate-extraction.md` replaces the ownerless manifest gap with the new manifest link while preserving `workspace-internal` readiness.

## Commands

- `openspec validate document-config-plugin-extraction-contract --strict`
- `git diff --check`
