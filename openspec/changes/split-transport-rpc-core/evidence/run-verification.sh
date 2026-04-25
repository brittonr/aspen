#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
CHANGE_DIR="$ROOT/openspec/changes/split-transport-rpc-core"
EVIDENCE_DIR="$CHANGE_DIR/evidence"
TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/aspen-split-transport-rpc-verification-target}"
mkdir -p "$EVIDENCE_DIR" "$TARGET_DIR"

run_capture() {
  local name="$1"
  shift
  local out="$EVIDENCE_DIR/${name}.txt"
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

metadata_capture() {
  local manifest="$1"
  local output="$2"
  cargo metadata --format-version 1 --manifest-path "$manifest" > "$output"
}

forbidden_grep_absent() {
  local metadata="$1"
  local output="$2"
  python - "$metadata" "$output" <<'PY'
import json
import sys
metadata_path, output_path = sys.argv[1], sys.argv[2]
forbidden = {
    "aspen",
    "aspen-cluster",
    "aspen-sharding",
    "aspen-trust",
    "aspen-raft",
    "aspen-rpc-handlers",
    "aspen-core-essentials-handler",
    "aspen-blob-handler",
    "aspen-ci-handler",
    "aspen-cluster-handler",
    "aspen-forge-handler",
    "aspen-job-handler",
    "aspen-nix-handler",
    "aspen-secrets-handler",
}
data = json.load(open(metadata_path))
found = sorted({pkg["name"] for pkg in data.get("packages", []) if pkg["name"] in forbidden})
with open(output_path, "w") as out:
    if found:
        out.write("Forbidden package names found:\n")
        for name in found:
            out.write(f"- {name}\n")
        sys.exit(1)
    out.write("No forbidden runtime package names found in metadata packages.\n")
PY
}

run_capture v1-transport-default-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-transport --no-default-features
run_capture v1-transport-runtime-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-transport --features runtime
run_capture v1-transport-default-cargo-tree cargo tree -p aspen-transport -e normal
run_capture v1-transport-source-audit rg 'aspen_core|aspen_auth|aspen_sharding|aspen_trust|openraft|irpc|aspen_raft' crates/aspen-transport/src

run_capture v2-rpc-core-default-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-rpc-core --no-default-features
run_capture v2-rpc-core-runtime-cargo-check env CARGO_TARGET_DIR="$TARGET_DIR" cargo check -p aspen-rpc-core --features runtime-context
run_capture v2-rpc-core-default-cargo-tree cargo tree -p aspen-rpc-core -e normal
run_capture v2-rpc-core-source-audit rg 'aspen_core|aspen_auth|aspen_raft|aspen_transport|aspen_sharding|aspen_coordination|aspen_cluster|aspen_jobs|aspen_hooks|aspen_blob|aspen_forge|aspen_ci|aspen_testing' crates/aspen-rpc-core/src

run_capture i7-downstream-transport-tests env CARGO_TARGET_DIR="$TARGET_DIR" cargo test --manifest-path openspec/changes/split-transport-rpc-core/fixtures/downstream-transport/Cargo.toml
metadata_capture openspec/changes/split-transport-rpc-core/fixtures/downstream-transport/Cargo.toml "$EVIDENCE_DIR/i7-downstream-transport-metadata.json"
forbidden_grep_absent "$EVIDENCE_DIR/i7-downstream-transport-metadata.json" "$EVIDENCE_DIR/i7-downstream-transport-forbidden-grep.txt"

run_capture i7-downstream-rpc-tests env CARGO_TARGET_DIR="$TARGET_DIR" cargo test --manifest-path openspec/changes/split-transport-rpc-core/fixtures/downstream-rpc-core/Cargo.toml
metadata_capture openspec/changes/split-transport-rpc-core/fixtures/downstream-rpc-core/Cargo.toml "$EVIDENCE_DIR/i7-downstream-rpc-metadata.json"
forbidden_grep_absent "$EVIDENCE_DIR/i7-downstream-rpc-metadata.json" "$EVIDENCE_DIR/i7-downstream-rpc-forbidden-grep.txt"

cat > "$EVIDENCE_DIR/verification-summary.md" <<SUMMARY
# Split transport/RPC verification summary

- Target dir: \`$TARGET_DIR\`
- Transport default and runtime checks: \`v1-transport-default-cargo-check.txt\`, \`v1-transport-runtime-cargo-check.txt\`
- RPC core default and runtime checks: \`v2-rpc-core-default-cargo-check.txt\`, \`v2-rpc-core-runtime-cargo-check.txt\`
- Downstream fixture checks: \`i7-downstream-transport-tests.txt\`, \`i7-downstream-rpc-tests.txt\`
- Downstream metadata and forbidden-grep artifacts were generated for both fixtures.
SUMMARY
