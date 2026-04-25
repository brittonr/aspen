#!/usr/bin/env bash
set -uo pipefail

change=extract-blob-castore-cache
evidence="openspec/changes/${change}/evidence"
logs="${evidence}/r1-baseline-logs"
summary="${evidence}/r1-baseline.md"

rm -rf "${logs}"
mkdir -p "${logs}"
: > "${summary}"

{
  echo "# R1 baseline: blob/castore/cache extraction"
  echo
  echo "Captured: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "## Worktree setup"
  echo
  echo "- Cargo path dependencies use local .pi/worktrees sibling mirrors for aspen-dns and aspen-wasm-plugin so their Aspen path dependencies resolve to this session worktree."
  echo
  echo "## Scope"
  echo
  echo "Core crates: aspen-blob, aspen-castore, aspen-cache."
  echo "Representative consumers: aspen-rpc-core --features blob, aspen-blob-handler, aspen-snix, aspen-snix-bridge, aspen-nix-cache-gateway, aspen-ci-executor-nix."
  echo
  echo "## Command matrix"
  echo
  echo "| Command | Exit | Artifact |"
  echo "|---|---:|---|"
} >> "${summary}"

run_cmd() {
  local name="$1"
  shift
  local log="${logs}/${name}.txt"
  echo "COMMAND: $*" > "${log}"
  echo "START: $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "${log}"
  "$@" >> "${log}" 2>&1
  local code=$?
  echo "END: $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "${log}"
  echo "EXIT_CODE: ${code}" >> "${log}"
  printf "| %s | %s | %s |\n" "$*" "${code}" "${log}" >> "${summary}"
  return 0
}

run_cmd cargo-check-aspen-blob cargo check -p aspen-blob --locked
run_cmd cargo-check-aspen-castore cargo check -p aspen-castore --locked
run_cmd cargo-check-aspen-cache cargo check -p aspen-cache --locked
run_cmd cargo-check-aspen-rpc-core-blob cargo check -p aspen-rpc-core --features blob --locked
run_cmd cargo-check-aspen-blob-handler cargo check -p aspen-blob-handler --locked
run_cmd cargo-check-aspen-snix cargo check -p aspen-snix --locked
run_cmd cargo-check-aspen-snix-bridge cargo check -p aspen-snix-bridge --locked
run_cmd cargo-check-aspen-nix-cache-gateway cargo check -p aspen-nix-cache-gateway --locked
run_cmd cargo-check-aspen-ci-executor-nix cargo check -p aspen-ci-executor-nix --locked

for pkg in aspen-blob aspen-castore aspen-cache aspen-rpc-core aspen-blob-handler aspen-snix aspen-snix-bridge aspen-nix-cache-gateway aspen-ci-executor-nix; do
  safe=${pkg//-/_}
  run_cmd "cargo-tree-${safe}-normal" cargo tree -p "${pkg}" -e normal --depth 3 --locked
  run_cmd "cargo-tree-${safe}-features" cargo tree -p "${pkg}" -e features --depth 3 --locked
done

python - <<'PY' >> "${summary}"
import tomllib
from pathlib import Path

scope = ["aspen-blob", "aspen-castore", "aspen-cache"]
backend = {
    "iroh",
    "iroh-blobs",
    "snix-castore",
    "snix-store",
    "nix-compat",
    "ed25519-dalek",
    "sha2",
    "data-encoding",
    "rand_core_06",
    "irpc",
}
reusable = {
    "serde",
    "serde_json",
    "postcard",
    "snafu",
    "thiserror",
    "anyhow",
    "async-trait",
    "async-stream",
    "futures",
    "bytes",
    "tokio",
    "tokio-util",
    "n0-future",
    "n0-error",
    "parking_lot",
    "tracing",
    "hex",
    "prost",
    "aspen-kv-types",
    "aspen-blob",
}
adapter = {
    "aspen-core",
    "aspen-client-api",
    "aspen-cache",
    "aspen-castore",
    "aspen-snix",
    "aspen-client",
    "aspen-transport",
}
forbidden_prefixes = (
    "aspen-rpc",
    "aspen-blob-handler",
    "aspen-ci-handler",
    "aspen-job-handler",
    "aspen-nix-handler",
    "aspen-cluster",
    "aspen-cli",
    "aspen-dogfood",
    "aspen-forge-web",
    "aspen-tui",
    "aspen-testing",
)

print("\n## Direct dependency classification\n")
for manifest in sorted(Path("crates").glob("*/Cargo.toml")):
    data = tomllib.loads(manifest.read_text())
    pkg = data.get("package", {}).get("name")
    if pkg not in scope:
        continue
    print(f"### {pkg}\n")
    print("| Section | Dependency | Class | Notes |")
    print("|---|---|---|---|")
    for section in ["dependencies", "dev-dependencies", "build-dependencies"]:
        for name, value in sorted(data.get(section, {}).items()):
            dep_pkg = value.get("package", name) if isinstance(value, dict) else name
            if section != "dependencies":
                cls = "test-only"
                note = "non-production dependency"
            elif dep_pkg in backend or name in backend:
                cls = "backend-purpose"
                note = "declared backend/domain dependency"
            elif dep_pkg in reusable or name in reusable:
                cls = "reusable-domain"
                note = "library/runtime utility or reusable Aspen domain dependency"
            elif dep_pkg in adapter or name in adapter:
                cls = "adapter-runtime"
                note = "Aspen integration coupling to evaluate/gate"
            elif dep_pkg.startswith(forbidden_prefixes) or name.startswith(forbidden_prefixes):
                cls = "forbidden"
                note = "app shell/handler/testing dependency should not be in reusable default"
            else:
                cls = "reusable-domain"
                note = "unclassified external utility; verify during cleanup"
            print(f"| {section} | {name} | {cls} | {note} |")
    print()

print("## Baseline coupling observations\n")
print("- aspen-blob default currently depends on aspen-client-api for replication RPC and aspen-core for KV-backed metadata; this is the primary adapter/runtime coupling for I3.")
print("- aspen-castore default currently depends on aspen-core for circuit-breaker behavior; this is the I4 seam.")
print("- aspen-cache default currently depends on aspen-core, aspen-kv-types, and aspen-blob; dev tests pull aspen-testing, which should remain test-only or move to shell integration coverage.")
print("- Representative consumer compile/check status is recorded in the command matrix above; failures are baseline facts, not regressions from this task.")
PY
