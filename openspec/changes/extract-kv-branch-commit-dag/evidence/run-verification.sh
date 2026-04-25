#!/usr/bin/env bash
set -u
change=openspec/changes/extract-kv-branch-commit-dag
evidence="$change/evidence"
mkdir -p "$evidence"
run() {
  local out="$1"; shift
  echo "Command: $*" > "$evidence/$out"
  "$@" >> "$evidence/$out" 2>&1
  local code=$?
  echo "Exit: $code" >> "$evidence/$out"
  echo "$out $code"
  return 0
}
run v1-cargo-check-aspen-commit-dag.txt cargo check -p aspen-commit-dag
run v1-cargo-check-aspen-kv-branch-default.txt cargo check -p aspen-kv-branch
run v1-cargo-check-aspen-kv-branch-no-default.txt cargo check -p aspen-kv-branch --no-default-features
run v1-cargo-check-aspen-kv-branch-commit-dag.txt cargo check -p aspen-kv-branch --features commit-dag
run v1-cargo-tree-aspen-commit-dag.txt cargo tree -p aspen-commit-dag --edges normal
run v1-cargo-tree-aspen-kv-branch-default.txt cargo tree -p aspen-kv-branch --edges normal
run v1-cargo-tree-aspen-kv-branch-no-default.txt cargo tree -p aspen-kv-branch --no-default-features --edges normal
run v1-cargo-tree-aspen-kv-branch-commit-dag.txt cargo tree -p aspen-kv-branch --features commit-dag --edges normal
run v1-source-raft-import-grep.txt bash -c "if rg -n 'aspen_raft::verified|aspen-raft =|aspen_raft::' crates/aspen-commit-dag/Cargo.toml crates/aspen-commit-dag/src; then exit 1; else echo 'No aspen-raft source/Cargo imports found.'; fi"
run v1-commit-dag-forbidden-grep.txt bash -c "cargo tree -p aspen-commit-dag --edges normal --prefix none | awk '{print \$1}' | rg -n '^(aspen-raft|aspen|aspen-core-shell|aspen-cluster|aspen-rpc-core|aspen-rpc-handlers|aspen-transport|aspen-raft-network|iroh|iroh-base|irpc|aspen-trust|aspen-secrets|aspen-sql)$' && exit 1 || echo 'No forbidden packages found.'"
run v1-kv-branch-default-forbidden-grep.txt bash -c "cargo tree -p aspen-kv-branch --edges normal --prefix none | awk '{print \$1}' | rg -n '^(aspen-commit-dag|aspen-raft|aspen|aspen-core-shell|aspen-cluster|aspen-rpc-core|aspen-rpc-handlers|aspen-transport|aspen-raft-network|iroh|iroh-base|irpc|aspen-trust|aspen-secrets|aspen-sql)$' && exit 1 || echo 'No forbidden packages found.'"
run v1-kv-branch-commit-dag-forbidden-grep.txt bash -c "cargo tree -p aspen-kv-branch --features commit-dag --edges normal --prefix none | awk '{print \$1}' | rg -n '^(aspen-raft|aspen|aspen-core-shell|aspen-cluster|aspen-rpc-core|aspen-rpc-handlers|aspen-transport|aspen-raft-network|iroh|iroh-base|irpc|aspen-trust|aspen-secrets|aspen-sql)$' && exit 1 || echo 'No forbidden packages found.'"
run v2-nextest-aspen-commit-dag.txt cargo nextest run -p aspen-commit-dag
run v2-nextest-aspen-kv-branch-commit-dag.txt cargo nextest run -p aspen-kv-branch --features commit-dag
run v3-cargo-check-aspen-jobs-kv-branch.txt cargo check -p aspen-jobs --features kv-branch
run v3-cargo-check-aspen-ci-executor-shell-kv-branch.txt cargo check -p aspen-ci-executor-shell --features kv-branch
run v3-cargo-check-aspen-deploy-kv-branch.txt cargo check -p aspen-deploy --features kv-branch
run v3-cargo-check-aspen-fuse-kv-branch.txt cargo check -p aspen-fuse --features kv-branch
run v3-cargo-check-aspen-docs-commit-dag-federation.txt cargo check -p aspen-docs --features commit-dag-federation
run v3-cargo-check-aspen-cli-commit-dag.txt cargo check -p aspen-cli --features commit-dag
