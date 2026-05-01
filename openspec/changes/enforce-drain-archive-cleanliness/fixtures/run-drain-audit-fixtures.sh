#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../../../.." >/dev/null && pwd)
AUDIT="$REPO_ROOT/scripts/openspec-drain-audit.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

run_expect_success() {
  local name="$1"
  shift
  local out="$TMP_DIR/$name.out"
  if ! "$AUDIT" "$@" >"$out" 2>&1; then
    echo "FAIL: expected success for $name" >&2
    cat "$out" >&2
    exit 1
  fi
  if ! grep -q 'OK: OpenSpec drain archive cleanliness audit passed' "$out"; then
    echo "FAIL: success case did not print OK marker for $name" >&2
    cat "$out" >&2
    exit 1
  fi
  cat "$out"
}

run_expect_failure() {
  local name="$1"
  local expected="$2"
  shift 2
  local out="$TMP_DIR/$name.out"
  if "$AUDIT" "$@" >"$out" 2>&1; then
    echo "FAIL: expected failure for $name" >&2
    cat "$out" >&2
    exit 1
  fi
  if ! grep -q "$expected" "$out"; then
    echo "FAIL: failure case $name did not contain expected text: $expected" >&2
    cat "$out" >&2
    exit 1
  fi
  cat "$out"
}

clean_root="$TMP_DIR/clean/openspec/changes"
mkdir -p "$clean_root/archive/2026-04-30-completed-change"
run_expect_success clean-drain \
  --repo-root "$REPO_ROOT" \
  --changes-root "$clean_root" \
  --archive "$clean_root/archive/2026-04-30-completed-change"

leftover_root="$TMP_DIR/leftover/openspec/changes"
mkdir -p "$leftover_root/archive/2026-04-30-completed-change" "$leftover_root/stale-active-change"
run_expect_failure leftover-active 'active OpenSpec change directories remain' \
  --repo-root "$REPO_ROOT" \
  --changes-root "$leftover_root" \
  --archive "$leftover_root/archive/2026-04-30-completed-change"

state_root="$TMP_DIR/state/openspec/changes"
mkdir -p "$state_root/archive/2026-04-30-completed-change"
printf 'stale drain state\n' > "$state_root/.drain-state.md"
run_expect_failure stale-drain-state 'drain state file remains' \
  --repo-root "$REPO_ROOT" \
  --changes-root "$state_root" \
  --archive "$state_root/archive/2026-04-30-completed-change"

bad_archive_root="$TMP_DIR/bad-archive/openspec/changes"
mkdir -p "$bad_archive_root/archive/2026-04-30-completed-change" "$TMP_DIR/not-under-archive"
run_expect_failure archive-path 'archive path is not under' \
  --repo-root "$REPO_ROOT" \
  --changes-root "$bad_archive_root" \
  --archive "$TMP_DIR/not-under-archive"
