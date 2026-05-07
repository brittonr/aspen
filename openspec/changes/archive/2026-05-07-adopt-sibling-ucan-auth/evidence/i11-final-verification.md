# I11 final UCAN adoption verification

- Change: `adopt-sibling-ucan-auth`
- Task: Run targeted Rust tests, dependency graph checks, Nix/source-boundary checks, OpenSpec validation, helper verification, and `git diff --check` before archive.
- Started: 2026-05-07T00:11:50Z
- Completed: 2026-05-07T00:13:01Z
- Status: PASS

## Commands

- command: `CARGO_TARGET_DIR=target/agent cargo test -p aspen-auth --all-targets`
  - result: pass; 88 tests passed.
- command: `CARGO_TARGET_DIR=target/agent cargo check -p aspen-auth-core --no-default-features`
  - result: pass.
- command: `CARGO_TARGET_DIR=target/agent cargo check -p aspen-auth --all-targets`
  - result: pass.
- command: `CARGO_TARGET_DIR=target/agent cargo tree -p aspen-auth-core --no-default-features --edges normal --prefix none > /tmp/aspen-auth-core-tree-final.txt`
  - result: pass.
- command: `CARGO_TARGET_DIR=target/agent cargo tree -p aspen-auth --edges normal --prefix none > /tmp/aspen-auth-tree-final.txt`
  - result: pass.
- command: `CARGO_TARGET_DIR=target/agent cargo tree -p aspen-core --no-default-features --edges normal --prefix none > /tmp/aspen-core-nodefault-tree-final.txt`
  - result: pass.
- command: `CARGO_TARGET_DIR=target/agent cargo tree -p aspen-auth-core --no-default-features -e features --prefix none > /tmp/aspen-auth-core-features-final.txt`
  - result: pass.
- command: `CARGO_TARGET_DIR=target/agent cargo tree -p aspen-core --no-default-features -e features --prefix none > /tmp/aspen-core-nodefault-features-final.txt`
  - result: pass.
- command: `python scripts/check-aspen-core-no-std-boundary.py --manifest-path crates/aspen-core/Cargo.toml --allowlist scripts/aspen-core-no-std-transitives.txt --output /tmp/aspen-core-no-std-current-final.txt --diff-output /tmp/aspen-core-no-std-diff-final.txt`
  - result: pass.
- command: dependency-boundary assertion script over final `/tmp/*-final.txt` tree outputs
  - result: pass; `aspen-auth-core` includes `ucan-core` but not root `ucan`/`verified-logic`, runtime `aspen-auth` includes root `ucan`/`verified-logic`, and protected `aspen-core --no-default-features` excludes Aspen auth and UCAN dependencies.
- command: `openspec validate adopt-sibling-ucan-auth --strict`
  - result: pass; `Change 'adopt-sibling-ucan-auth' is valid`.
- command: `git diff --check`
  - result: pass.

## Archive readiness

All retained tasks are complete. The active change is ready for helper verification and archive after this evidence and task bookkeeping are committed.
