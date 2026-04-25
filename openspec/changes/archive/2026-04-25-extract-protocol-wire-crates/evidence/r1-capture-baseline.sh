#!/usr/bin/env bash
set -u
change=openspec/changes/extract-protocol-wire-crates
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
for pkg in aspen-client-api aspen-forge-protocol aspen-jobs-protocol aspen-coordination-protocol; do
  run "cargo-check-$pkg" cargo check -p "$pkg"
  run "cargo-check-$pkg-no-default" cargo check -p "$pkg" --no-default-features
  run "cargo-tree-$pkg" cargo tree -p "$pkg" --edges normal
  run "cargo-tree-$pkg-no-default" cargo tree -p "$pkg" --no-default-features --edges normal
  run "tests-$pkg" cargo test -p "$pkg"
done
run client-api-tests-nextest cargo nextest run -p aspen-client-api
cat > "$evidence/r1-baseline.md" <<'MD'
# R1 Baseline

Baseline protocol/wire compile, dependency, and compatibility-test evidence captured under `r1-baseline-logs/`.

Observed shape:
- `aspen-client-api` depends on portable `aspen-auth-core` through dependency key `aspen-auth` plus protocol crates.
- Forge/jobs/coordination protocol crates default to serialization-only dependencies.
- Existing `aspen-client-api` tests already pin postcard discriminants for client request/response variants.
MD
