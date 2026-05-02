# Verification

Evidence captured under `openspec/changes/complete-secrets-kv-traits-boundary/evidence/` before archive.

- `cargo check -p aspen-secrets --no-default-features`
  - Evidence: `evidence/aspen-secrets-no-default-check.txt`
- `cargo test -p aspen-secrets --no-default-features --no-fail-fast`
  - Evidence: `evidence/aspen-secrets-no-default-test.txt`
- `cargo check -p aspen-secrets --features trust`
  - Evidence: `evidence/aspen-secrets-trust-check.txt`
- `cargo check -p aspen-secrets-handler --no-default-features`
  - Evidence: `evidence/aspen-secrets-handler-no-default-check.txt`
- `cargo check --bin aspen-node --features node-runtime-apps,blob,automerge,secrets,forge`
  - Evidence: `evidence/aspen-node-secrets-check.txt`
- `cargo tree -p aspen-secrets --no-default-features -e normal` plus forbidden dependency grep
  - Evidence: `evidence/aspen-secrets-no-default-tree.txt` and `evidence/forbidden-runtime-dependency-check.txt`
- `rg` source check for stale `aspen_core::` use in `crates/aspen-secrets`
  - Evidence: `evidence/aspen-secrets-aspen-core-source-grep.txt`
