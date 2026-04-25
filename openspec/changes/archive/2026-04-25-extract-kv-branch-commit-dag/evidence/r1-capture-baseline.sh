#!/usr/bin/env bash
set -u
change=openspec/changes/extract-kv-branch-commit-dag
evidence="$change/evidence"
mkdir -p "$evidence/r1-baseline-logs"
run() {
  local name="$1"; shift
  local out="$evidence/r1-baseline-logs/$name.txt"
  echo "Command: $*" > "$out"
  "$@" >> "$out" 2>&1
  local code=$?
  echo "Exit: $code" >> "$out"
  echo "$name $code"
  return 0
}
run cargo-check-aspen-commit-dag cargo check -p aspen-commit-dag
run cargo-check-aspen-kv-branch cargo check -p aspen-kv-branch
run cargo-check-aspen-kv-branch-no-default cargo check -p aspen-kv-branch --no-default-features
run cargo-check-aspen-kv-branch-commit-dag cargo check -p aspen-kv-branch --features commit-dag
run cargo-tree-aspen-commit-dag cargo tree -p aspen-commit-dag --edges normal
run cargo-tree-aspen-kv-branch cargo tree -p aspen-kv-branch --edges normal
run cargo-tree-aspen-kv-branch-no-default cargo tree -p aspen-kv-branch --no-default-features --edges normal
run cargo-tree-aspen-kv-branch-commit-dag cargo tree -p aspen-kv-branch --features commit-dag --edges normal
run imports-commit-dag rg -n 'aspen_raft::verified|aspen-raft' crates/aspen-commit-dag
run imports-kv-branch rg -n 'aspen_raft::verified|aspen-raft|aspen_commit_dag|aspen-commit-dag' crates/aspen-kv-branch
cat > "$evidence/r1-baseline.md" <<'MD'
# R1 Baseline

Baseline commands captured under `r1-baseline-logs/`.

Expected current blocker before implementation:
- `aspen-commit-dag` has a normal `aspen-raft` dependency and source imports from `aspen_raft::verified`.
- `aspen-kv-branch` keeps `commit-dag` behind its named feature.
MD
