# Verification

## Evidence

- Inventory source: `crates/aspen-nickel/Cargo.toml` shows default dependencies on `nickel-lang`, `aspen-core`, serde/serde_json, SNAFU/thiserror, and tracing.
- Inventory source: `crates/aspen-plugin-api/Cargo.toml` shows a leaf default graph over `serde`, `serde_json`, and `semver` with no Aspen workspace dependencies.
- Changed file: `docs/crate-extraction/config-plugin.md` documents owner, feature contract, dependency decisions, representative consumers, exceptions, and verification rails.
- Changed file: `docs/crate-extraction.md` replaces the ownerless manifest gap with the new manifest link while preserving `workspace-internal` readiness.
- Changed file: `docs/crate-extraction/policy.ncl` adds `aspen_nickel` and `aspen_plugin_api` policy candidates while keeping both `workspace-internal`.
- Changed file: `scripts/check-crate-extraction-readiness.rs` recognizes the `config-plugin` family and its required evidence files.
- Changed file: `scripts/check-config-plugin-examples.rs` runs standalone plugin API and Nickel config examples.

## Task Coverage

- Task: Add policy/inventory rows while keeping readiness `workspace-internal`.
  - Evidence: `docs/crate-extraction/policy.ncl` candidates `aspen_nickel` and `aspen_plugin_api`; broader inventory row remains `workspace-internal`.
- Task: Add or document standalone example checks for config parsing and plugin API protocol/types.
  - Evidence: `openspec/changes/document-config-plugin-extraction-contract/evidence/config-plugin-standalone-examples.txt` from `scripts/check-config-plugin-examples.rs`.
- Task: Add checker expectations for missing manifest/owner/evidence failures if the family is later promoted.
  - Evidence: `scripts/check-crate-extraction-readiness.rs` maps `config-plugin` to policy candidates, inventory row, and evidence artifacts; `openspec/changes/document-config-plugin-extraction-contract/evidence/config-plugin-readiness.md` captures the passing checker report.
- Task: Run the manifest/readiness checker, strict OpenSpec validation, and `git diff --check`.
  - Evidence: command list below plus generated readiness report.

## Commands

- `nix develop -c cargo -q -Zscript scripts/check-config-plugin-examples.rs`
- `cargo tree -p aspen-plugin-api -e normal`
- `cargo tree -p aspen-nickel -e normal`
- `cargo test -p aspen-plugin-api -- --nocapture`
- `cargo check -p aspen-cli --features plugins-rpc`
- `cargo check -p aspen-cluster --features nickel`
- `cargo check -p aspen-ci --features nickel`
- `nix develop -c cargo -q -Zscript scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family config-plugin --output-json openspec/changes/document-config-plugin-extraction-contract/evidence/config-plugin-readiness.json --output-markdown openspec/changes/document-config-plugin-extraction-contract/evidence/config-plugin-readiness.md`
- `openspec validate document-config-plugin-extraction-contract --strict`
- `openspec validate --all --strict --json`
- `git diff --check`
