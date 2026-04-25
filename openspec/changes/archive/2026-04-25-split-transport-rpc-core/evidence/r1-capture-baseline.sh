#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
CHANGE_DIR="$ROOT/openspec/changes/split-transport-rpc-core"
OUT_DIR="$CHANGE_DIR/evidence/r1-baseline-logs"
TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/aspen-split-transport-rpc-r1-target}"
mkdir -p "$OUT_DIR" "$TARGET_DIR"

run_capture() {
  local name="$1"
  shift
  local out="$OUT_DIR/${name}.txt"
  {
    printf '$'
    printf ' %q' "$@"
    printf '\n'
    "$@"
    printf '\nExit: 0\n'
  } >"$out" 2>&1 || {
    local code=$?
    printf '\nExit: %s\n' "$code" >>"$out"
    return "$code"
  }
}

run_capture git-status git status --short
run_capture transport-manifest sed -n '1,220p' crates/aspen-transport/Cargo.toml
run_capture rpc-core-manifest sed -n '1,240p' crates/aspen-rpc-core/Cargo.toml
run_capture transport-source-imports rg 'aspen_|openraft|iroh|irpc|postcard|metrics|parking_lot|tokio_stream' crates/aspen-transport/src
run_capture rpc-core-source-imports rg 'aspen_|openraft|iroh|irpc|metrics|tokio' crates/aspen-rpc-core/src
run_capture transport-cargo-tree cargo tree -p aspen-transport -e normal
run_capture rpc-core-cargo-tree cargo tree -p aspen-rpc-core -e normal
run_capture transport-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-transport
run_capture rpc-core-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-rpc-core
run_capture raft-network-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-raft-network
run_capture raft-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-raft
run_capture cluster-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-cluster --no-default-features --features bootstrap
run_capture client-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-client
run_capture rpc-handlers-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-rpc-handlers

cat > "$CHANGE_DIR/evidence/r1-baseline.md" <<SUMMARY
# R1 Baseline: split transport/RPC core

- Target dir: \`$TARGET_DIR\`
- Transport default compile: see \`evidence/r1-baseline-logs/transport-cargo-check.txt\`
- RPC core default compile: see \`evidence/r1-baseline-logs/rpc-core-cargo-check.txt\`
- Representative consumer compiles: raft-network, raft, cluster bootstrap, client, rpc-handlers logs under \`evidence/r1-baseline-logs/\`
- Dependency trees: \`transport-cargo-tree.txt\`, \`rpc-core-cargo-tree.txt\`
- Source import audits: \`transport-source-imports.txt\`, \`rpc-core-source-imports.txt\`

## Baseline classification

- \`aspen-transport\` default currently includes generic Iroh stream helpers and runtime Raft/log-subscriber/trust/sharding/auth integrations in one crate manifest.
- \`aspen-rpc-core\` default currently includes handler traits plus concrete \`ClientProtocolContext\` service bundles that reference Raft, transport, sharding, coordination, hooks, auth, metrics, and optional domain crates.
- Representative consumers compile against implicit normal dependencies before the split; the implementation must preserve runtime behavior by adding explicit feature opt-ins.
SUMMARY
