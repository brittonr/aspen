#!/usr/bin/env bash
set -u
change=openspec/changes/extract-protocol-wire-crates
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
for pkg in aspen-client-api aspen-forge-protocol aspen-jobs-protocol aspen-coordination-protocol; do
  run "v1-cargo-check-$pkg.txt" cargo check -p "$pkg"
  run "v1-cargo-check-$pkg-no-default.txt" cargo check -p "$pkg" --no-default-features
  run "v2-cargo-tree-$pkg.txt" cargo tree -p "$pkg" --edges normal
  run "v2-cargo-tree-$pkg-no-default.txt" cargo tree -p "$pkg" --no-default-features --edges normal
  run "v3-cargo-test-$pkg.txt" cargo test -p "$pkg"
done
run v1-cargo-check-client-api-wasm.txt cargo check -p aspen-client-api --target wasm32-unknown-unknown
run v2-client-api-forbidden-grep.txt bash -c "cargo tree -p aspen-client-api --edges normal --prefix none | awk '{print \$1}' | rg -n '^(aspen|aspen-raft|aspen-core-shell|aspen-cluster|aspen-rpc-core|aspen-rpc-handlers|aspen-transport|aspen-raft-network|aspen-auth|iroh|irpc|aspen-trust|aspen-secrets|aspen-sql|aspen-cli|aspen-tui|aspen-dogfood|aspen-.*-handler)$' && exit 1 || echo 'No forbidden packages found; iroh-base is documented as auth-core key-only exception.'"
for pkg in aspen-forge-protocol aspen-jobs-protocol aspen-coordination-protocol; do
  run "v2-$pkg-forbidden-grep.txt" bash -c "cargo tree -p $pkg --edges normal --prefix none | awk '{print \$1}' | rg -n '^(aspen|aspen-raft|aspen-core-shell|aspen-cluster|aspen-rpc-core|aspen-rpc-handlers|aspen-transport|aspen-raft-network|aspen-auth|iroh|iroh-base|irpc|aspen-trust|aspen-secrets|aspen-sql|aspen-cli|aspen-tui|aspen-dogfood|aspen-.*-handler)$' && exit 1 || echo 'No forbidden packages found.'"
done
cp "$evidence/v3-cargo-test-aspen-client-api.txt" "$evidence/i3-client-api-compatibility-tests.txt"
