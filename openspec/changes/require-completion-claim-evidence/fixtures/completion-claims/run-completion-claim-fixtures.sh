#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../../../../.." >/dev/null && pwd)
CHECKER="$REPO_ROOT/scripts/check-completion-claim-evidence.py"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

write_file() {
  local path="$1"
  shift
  printf '%s\n' "$@" > "$path"
}

run_pass() {
  local name="$1" response="$2" evidence="$3"
  local out="$TMP_DIR/$name.out"
  if ! "$CHECKER" --response "$response" --evidence "$evidence" >"$out" 2>&1; then
    echo "FAIL: expected pass for $name" >&2
    cat "$out" >&2
    exit 1
  fi
  cat "$out"
}

run_fail() {
  local name="$1" response="$2" evidence="$3" expected="$4"
  local out="$TMP_DIR/$name.out"
  if "$CHECKER" --response "$response" --evidence "$evidence" >"$out" 2>&1; then
    echo "FAIL: expected failure for $name" >&2
    cat "$out" >&2
    exit 1
  fi
  if ! grep -q "$expected" "$out"; then
    echo "FAIL: failure output for $name missing expected text: $expected" >&2
    cat "$out" >&2
    exit 1
  fi
  cat "$out"
}

resp_clean="$TMP_DIR/clean-response.txt"
evd_clean="$TMP_DIR/clean-evidence.txt"
write_file "$resp_clean" 'Working tree clean; validation passed.'
write_file "$evd_clean" 'git status --short' 'openspec validate require-completion-claim-evidence --json' '"valid": true'
run_pass clean-status-and-validation "$resp_clean" "$evd_clean"

resp_queue="$TMP_DIR/queue-response.txt"
evd_queue="$TMP_DIR/queue-evidence.txt"
write_file "$resp_queue" 'OpenSpec queue empty and archived as changes/archive/2026-05-01-example.'
write_file "$evd_queue" 'python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py drain-plan' 'active_changes: none' 'openspec archive example -y' 'Change archived as changes/archive/2026-05-01-example'
run_pass queue-empty-archived "$resp_queue" "$evd_queue"

resp_unsupported="$TMP_DIR/unsupported-response.txt"
evd_empty="$TMP_DIR/empty-evidence.txt"
write_file "$resp_unsupported" 'Working tree clean and OpenSpec queue empty.'
write_file "$evd_empty" 'No command transcript supplied.'
run_fail unsupported-claims "$resp_unsupported" "$evd_empty" 'unsupported completion claim'

resp_uncertain="$TMP_DIR/uncertain-response.txt"
write_file "$resp_uncertain" 'I did not run git status, so repo cleanliness is not verified.'
run_pass uncertain-summary "$resp_uncertain" "$evd_empty"

resp_checks="$TMP_DIR/checks-response.txt"
evd_checks="$TMP_DIR/checks-evidence.txt"
write_file "$resp_checks" 'Relevant checks passed.'
write_file "$evd_checks" 'scripts/openspec-preflight.sh require-completion-claim-evidence' 'OK: openspec/changes/require-completion-claim-evidence' 'exit 0'
run_pass checks-pass "$resp_checks" "$evd_checks"
