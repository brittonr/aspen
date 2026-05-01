#!/usr/bin/env bash
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
SCRIPT="$ROOT/scripts/check-openspec-delta-consistency.py"
FIXTURES="$ROOT/openspec/changes/detect-delta-spec-consistency/fixtures/delta-consistency"

cases=(
  valid-add
  missing-requirement-id
  missing-scenario-id
  valid-modify
  missing-modified-target
  valid-removal
  missing-removal-target
  migration-note
  feature-conflict
)

for case in "${cases[@]}"; do
  fixture="$FIXTURES/$case"
  expected_exit=$(sed -n 's/^exit=//p' "$fixture/expected.txt")
  expected_text=$(sed -n 's/^contains=//p' "$fixture/expected.txt")
  output_file="$fixture/output.txt"
  set +e
  "$SCRIPT" case --repo-root "$fixture" >"$output_file" 2>&1
  actual_exit=$?
  set -e
  printf '%s expected=%s actual=%s\n' "$case" "$expected_exit" "$actual_exit"
  if [[ "$actual_exit" != "$expected_exit" ]]; then
    cat "$output_file"
    exit 1
  fi
  if ! grep -Fq "$expected_text" "$output_file"; then
    echo "missing expected text for $case: $expected_text" >&2
    cat "$output_file" >&2
    exit 1
  fi
  sed 's/^/  /' "$output_file"
done
