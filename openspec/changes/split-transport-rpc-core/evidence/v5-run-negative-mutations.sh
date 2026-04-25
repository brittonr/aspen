#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
POLICY="$ROOT/docs/crate-extraction/policy.ncl"
RPC_CORE_MANIFEST="$ROOT/crates/aspen-rpc-core/Cargo.toml"
EVIDENCE_DIR="$ROOT/openspec/changes/split-transport-rpc-core/evidence"
READINESS_JSON="$EVIDENCE_DIR/v5-readiness.json"
READINESS_MD="$EVIDENCE_DIR/v5-readiness.md"
TMP_DIR="$(mktemp -d)"
cp "$POLICY" "$TMP_DIR/policy.ncl"
cp "$RPC_CORE_MANIFEST" "$TMP_DIR/aspen-rpc-core.Cargo.toml"

restore() {
  cp "$TMP_DIR/policy.ncl" "$POLICY"
  cp "$TMP_DIR/aspen-rpc-core.Cargo.toml" "$RPC_CORE_MANIFEST"
  rm -rf "$TMP_DIR"
}
trap restore EXIT

run_checker() {
  local json_out="$1"
  local md_out="$2"
  scripts/check-crate-extraction-readiness.rs \
    --policy docs/crate-extraction/policy.ncl \
    --inventory docs/crate-extraction.md \
    --manifest-dir docs/crate-extraction \
    --candidate-family transport-rpc \
    --output-json "$json_out" \
    --output-markdown "$md_out"
}

expect_failure() {
  local name="$1"
  shift
  local summary="$EVIDENCE_DIR/${name}-summary.txt"
  if "$@" > "$summary" 2>&1; then
    printf 'expected failure but command passed\n' >> "$summary"
    return 1
  fi
  printf '\nExpected failure observed.\n' >> "$summary"
}

run_checker "$READINESS_JSON" "$READINESS_MD"

python - <<'PY'
from pathlib import Path
p=Path('crates/aspen-rpc-core/Cargo.toml')
s=p.read_text()
needle='aspen-constants = { workspace = true }\n'
insert='aspen-rpc-handlers = { path = "../aspen-rpc-handlers" }\n'
if insert not in s:
    s=s.replace(needle, needle+insert, 1)
p.write_text(s)
PY
expect_failure v5-negative-unowned-runtime run_checker "$EVIDENCE_DIR/v5-negative-unowned-runtime.json" "$EVIDENCE_DIR/v5-negative-unowned-runtime.md"
cp "$TMP_DIR/aspen-rpc-core.Cargo.toml" "$RPC_CORE_MANIFEST"

python - <<'PY'
from pathlib import Path
p=Path('docs/crate-extraction/policy.ncl')
s=p.read_text()
s=s.replace('owner = "Aspen transport/RPC maintainers",\n          reason = "Iroh endpoint, connection, and stream types are the explicit reusable transport API."', 'owner = "owner needed",\n          reason = "Iroh endpoint, connection, and stream types are the explicit reusable transport API."', 1)
p.write_text(s)
PY
expect_failure v5-negative-missing-owner run_checker "$EVIDENCE_DIR/v5-negative-missing-owner.json" "$EVIDENCE_DIR/v5-negative-missing-owner.md"
cp "$TMP_DIR/policy.ncl" "$POLICY"

python - <<'PY'
from pathlib import Path
p=Path('docs/crate-extraction/policy.ncl')
s=p.read_text()
s=s.replace('aspen_rpc_core = {\n      class = "service library",\n      readiness_state = "workspace-internal",', 'aspen_rpc_core = {\n      class = "service library",\n      readiness_state = "publishable from monorepo",', 1)
p.write_text(s)
PY
expect_failure v5-negative-invalid-readiness run_checker "$EVIDENCE_DIR/v5-negative-invalid-readiness.json" "$EVIDENCE_DIR/v5-negative-invalid-readiness.md"
cp "$TMP_DIR/policy.ncl" "$POLICY"

missing_downstream="$EVIDENCE_DIR/i7-downstream-rpc-metadata.json"
mv "$missing_downstream" "$TMP_DIR/i7-downstream-rpc-metadata.json"
expect_failure v5-negative-missing-downstream run_checker "$EVIDENCE_DIR/v5-negative-missing-downstream.json" "$EVIDENCE_DIR/v5-negative-missing-downstream.md"
mv "$TMP_DIR/i7-downstream-rpc-metadata.json" "$missing_downstream"

missing_compat="$EVIDENCE_DIR/v4-compatibility-summary.txt"
mv "$missing_compat" "$TMP_DIR/v4-compatibility-summary.txt"
expect_failure v5-negative-missing-compatibility run_checker "$EVIDENCE_DIR/v5-negative-missing-compatibility.json" "$EVIDENCE_DIR/v5-negative-missing-compatibility.md"
mv "$TMP_DIR/v4-compatibility-summary.txt" "$missing_compat"

run_checker "$READINESS_JSON" "$READINESS_MD"
