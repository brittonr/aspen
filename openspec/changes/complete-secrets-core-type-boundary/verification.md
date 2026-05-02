# Verification

Evidence is captured in `evidence/` for the completed secrets core type boundary.

## Code and compatibility

- `aspen-secrets-core-check.txt`: `cargo check -p aspen-secrets-core --no-default-features`
- `aspen-secrets-core-test.txt`: `cargo test -p aspen-secrets-core --no-default-features --no-fail-fast`
- `aspen-secrets-no-default-check.txt`: `cargo check -p aspen-secrets --no-default-features`
- `aspen-secrets-no-default-test.txt`: `cargo test -p aspen-secrets --no-default-features --no-fail-fast`
- `aspen-secrets-handler-no-default-check.txt`: `cargo check -p aspen-secrets-handler --no-default-features`
- `aspen-secrets-handler-runtime-adapter-check.txt`: `cargo check -p aspen-secrets-handler --no-default-features --features runtime-adapter`
- `aspen-node-secrets-check.txt`: `cargo check --bin aspen-node --features node-runtime-apps,blob,automerge,secrets,forge`

## Boundary rails

- `aspen-secrets-core-no-default-tree.txt`: normal no-default dependency tree for `aspen-secrets-core`
- `forbidden-runtime-dependency-check.txt`: negative grep proving no runtime/transport/storage/auth dependencies in the core no-default normal tree

## OpenSpec and docs

- `openspec-helper-verify.txt`: file-based helper verification for this change
- `openspec-validate.txt`: `openspec validate complete-secrets-core-type-boundary --json`
- `markdownlint.txt`: markdownlint over touched docs and change docs
- `git-diff-check.txt`: `git diff --check`
