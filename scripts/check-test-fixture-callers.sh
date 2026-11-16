#!/usr/bin/env bash
# Fails when the fabricated passing Octet gate receipt is referenced outside test code.
#
# Feature gating hides the helper from `cargo build`, but Nix or other builds may enable
# `test-fixtures`. This source-level guard does not depend on features: besides the definition,
# references are allowed only in test files (`tests/`, `src/**/tests/`, `src/**/tests.rs`).
# Usage: check-test-fixture-callers.sh [SOURCE_ROOT]
set -euo pipefail

root="${1:-.}"
symbol=synthetic_clean_octet_gate_receipt_for_tests
definition='^src/octet/parts/gate/p002/body\.rs:[0-9]+:pub fn '"$symbol"'\('
test_paths='^(tests/|src/([^:]*/)?tests/|src/([^:]*/)?tests\.rs:)'

cd "$root"
dirs=()
for dir in src tests; do
  if [ -d "$dir" ]; then
    dirs+=("$dir")
  fi
done
if [ "${#dirs[@]}" -eq 0 ]; then
  echo "check-test-fixture-callers: no src/ or tests/ under $root" >&2
  exit 2
fi

violations="$(grep -rnF --include='*.rs' "$symbol" "${dirs[@]}" | grep -vE "$test_paths" | grep -vE "$definition" || true)"
if [ -n "$violations" ]; then
  echo "non-test references to $symbol (fabricated Octet evidence must stay test-only):" >&2
  echo "$violations" >&2
  exit 1
fi
echo "check-test-fixture-callers: no non-test references to $symbol"
