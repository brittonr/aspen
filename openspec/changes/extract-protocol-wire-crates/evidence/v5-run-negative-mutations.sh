#!/usr/bin/env bash
set -euo pipefail

change=openspec/changes/extract-protocol-wire-crates
evidence="$change/evidence"
policy=docs/crate-extraction/policy.ncl
inventory=docs/crate-extraction.md
manifest_dir=docs/crate-extraction
checker=scripts/check-crate-extraction-readiness.rs
family=protocol-wire

run_checker_expect_failure() {
  local name="$1"
  local expected="$2"
  local policy_path="$3"
  local report_json="$evidence/v5-negative-$name.json"
  local report_md="$evidence/v5-negative-$name.md"
  local stdout="$evidence/v5-negative-$name.stdout"
  local stderr="$evidence/v5-negative-$name.stderr"
  local summary="$evidence/v5-negative-$name-summary.txt"
  set +e
  "$checker" --policy "$policy_path" --inventory "$inventory" --manifest-dir "$manifest_dir" --candidate-family "$family" --output-json "$report_json" --output-markdown "$report_md" > "$stdout" 2> "$stderr"
  local code=$?
  set -e
  {
    echo "Command: $checker --policy $policy_path --inventory $inventory --manifest-dir $manifest_dir --candidate-family $family --output-json $report_json --output-markdown $report_md"
    echo "Exit: $code"
    echo "Expected substring: $expected"
  } > "$summary"
  if [[ "$code" -eq 0 ]]; then
    echo "negative mutation $name unexpectedly passed" >> "$summary"
    return 1
  fi
  if ! rg -n --fixed-strings "$expected" "$report_md" "$stderr" >> "$summary" 2>&1; then
    echo "negative mutation $name did not report expected failure" >> "$summary"
    return 1
  fi
}

with_temp_policy() {
  local name="$1"
  local expected="$2"
  local edit_kind="$3"
  local tmp_policy
  tmp_policy=$(mktemp --suffix=.ncl)
  cp "$policy" "$tmp_policy"
  python - "$tmp_policy" "$edit_kind" <<'PY'
from pathlib import Path
import re
import sys
path = Path(sys.argv[1])
edit_kind = sys.argv[2]
text = path.read_text()
if edit_kind == "missing-owner":
    text = text.replace(
        'dependency_path = "aspen-client-api -> aspen-auth-core",\n          owner = "Aspen protocol maintainers"',
        'dependency_path = "aspen-client-api -> aspen-auth-core",\n          owner = "owner needed"',
        1,
    )
elif edit_kind == "invalid-readiness":
    text = re.sub(r'(aspen_client_api = \{.*?readiness_state = )"workspace-internal"', r'\1"publishable from monorepo"', text, count=1, flags=re.S)
else:
    raise SystemExit(f"unknown edit kind: {edit_kind}")
path.write_text(text)
PY
  run_checker_expect_failure "$name" "$expected" "$tmp_policy"
  rm -f "$tmp_policy"
}

run_forbidden_dependency_mutation() {
  local backup
  backup=$(mktemp)
  cp crates/aspen-client-api/Cargo.toml "$backup"
  trap 'cp "$backup" crates/aspen-client-api/Cargo.toml; rm -f "$backup"' RETURN
  python - <<'PY'
from pathlib import Path
path = Path('crates/aspen-client-api/Cargo.toml')
text = path.read_text()
needle = '[dependencies]\n'
replacement = '[dependencies]\naspen-rpc-handlers = { workspace = true }\n'
if replacement not in text:
    text = text.replace(needle, replacement, 1)
path.write_text(text)
PY
  run_checker_expect_failure forbidden-runtime 'aspen_client_api: direct forbidden dependency `aspen-rpc-handlers`' "$policy"
}

run_missing_compatibility_rail_mutation() {
  local backup
  backup=$(mktemp)
  cp docs/crate-extraction/protocol-wire.md "$backup"
  trap 'cp "$backup" docs/crate-extraction/protocol-wire.md; rm -f "$backup"' RETURN
  python - <<'PY'
from pathlib import Path
path = Path('docs/crate-extraction/protocol-wire.md')
text = path.read_text().replace('## Compatibility and aliases', '## Wire contracts and aliases')
path.write_text(text)
PY
  run_checker_expect_failure missing-compatibility 'aspen_client_api: manifest missing `## Compatibility and aliases`' "$policy"
}

run_missing_downstream_evidence_mutation() {
  local tmp_change
  tmp_change=$(mktemp -d)
  mkdir -p "$tmp_change/evidence"
  cat > "$tmp_change/verification.md" <<'EOF'
# Verification Evidence

## Task Coverage

- [x] synthetic downstream evidence check
  - Evidence: intentionally missing downstream fixture artifacts
EOF
  local out_dir="$tmp_change/evidence"
  set +e
  "$checker" --policy "$policy" --inventory "$inventory" --manifest-dir "$manifest_dir" --candidate-family "$family" --output-json "$out_dir/report.json" --output-markdown "$out_dir/report.md" > "$evidence/v5-negative-missing-downstream.stdout" 2> "$evidence/v5-negative-missing-downstream.stderr"
  local code=$?
  set -e
  {
    echo "Command: $checker --policy $policy --inventory $inventory --manifest-dir $manifest_dir --candidate-family $family --output-json $out_dir/report.json --output-markdown $out_dir/report.md"
    echo "Exit: $code"
    echo "Expected substring: missing downstream fixture evidence"
    cat "$out_dir/report.md"
  } > "$evidence/v5-negative-missing-downstream-summary.txt"
  rm -rf "$tmp_change"
  if [[ "$code" -eq 0 ]]; then
    echo "negative mutation missing-downstream unexpectedly passed" >> "$evidence/v5-negative-missing-downstream-summary.txt"
    return 1
  fi
  rg -n --fixed-strings "missing downstream fixture evidence" "$evidence/v5-negative-missing-downstream-summary.txt" >/dev/null
}

run_forbidden_dependency_mutation
with_temp_policy missing-owner 'aspen_client_api: exception `aspen-client-api -> aspen-auth-core` has unassigned owner' missing-owner
with_temp_policy invalid-readiness 'aspen_client_api: forbidden readiness state `publishable from monorepo`' invalid-readiness
run_missing_compatibility_rail_mutation
run_missing_downstream_evidence_mutation

cat > "$evidence/v5-negative-mutations-summary.txt" <<'EOF'
Negative mutation checks passed:
- forbidden runtime dependency injection failed readiness checking
- missing exception owner failed readiness checking
- invalid readiness state failed readiness checking
- missing compatibility rail failed readiness checking
- missing downstream fixture evidence failed readiness checking
EOF
