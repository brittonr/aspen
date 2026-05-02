# Verification

Evidence is stored under `evidence/`.

## Commands

- `cargo check -p aspen-secrets-handler --no-default-features`
  - Evidence: `evidence/aspen-secrets-handler-no-default-check.txt`
- `cargo check -p aspen-secrets-handler --no-default-features --features runtime-adapter`
  - Evidence: `evidence/aspen-secrets-handler-runtime-adapter-check.txt`
- `cargo test -p aspen-secrets-handler --no-default-features --no-fail-fast`
  - Evidence: `evidence/aspen-secrets-handler-no-default-test.txt`
- `cargo check -p aspen-secrets --no-default-features`
  - Evidence: `evidence/aspen-secrets-no-default-check.txt`
- `cargo test -p aspen-secrets --no-default-features --no-fail-fast`
  - Evidence: `evidence/aspen-secrets-no-default-test.txt`
- `cargo check -p aspen-rpc-handlers --no-default-features --features secrets`
  - Evidence: `evidence/aspen-rpc-handlers-secrets-check.txt`
- `cargo check --bin aspen-node --features node-runtime-apps,blob,automerge,secrets,forge`
  - Evidence: `evidence/aspen-node-secrets-check.txt`
- negative dependency/source guards
  - Evidence: `evidence/forbidden-runtime-dependency-check.txt`, `evidence/aspen-secrets-handler-aspen-core-grep.txt`
- OpenSpec and Markdown validation
  - Evidence: `evidence/openspec-helper-verify.txt`, `evidence/openspec-validate.txt`, `evidence/markdownlint.txt`
