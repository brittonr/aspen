#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../../../../.." >/dev/null && pwd)
CHECKER="$REPO_ROOT/scripts/check-openspec-task-size.py"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

make_change() {
  local name="$1"
  mkdir -p "$TMP_DIR/$name"
  cat >"$TMP_DIR/$name/proposal.md" <<'PROPOSAL'
## Why
Fixture.
PROPOSAL
}

expect_pass() {
  local name="$1"
  "$CHECKER" "$TMP_DIR/$name" --repo-root "$TMP_DIR" >/tmp/task-size-check.out 2>&1 || {
    echo "unexpected failure for $name"
    cat /tmp/task-size-check.out
    return 1
  }
  echo "PASS: $name"
}

expect_fail() {
  local name="$1"
  local expected="$2"
  if "$CHECKER" "$TMP_DIR/$name" --repo-root "$TMP_DIR" >/tmp/task-size-check.out 2>&1; then
    echo "unexpected pass for $name"
    cat /tmp/task-size-check.out
    return 1
  fi
  if ! grep -Fq "$expected" /tmp/task-size-check.out; then
    echo "missing expected failure for $name: $expected"
    cat /tmp/task-size-check.out
    return 1
  fi
  echo "PASS: $name rejected: $expected"
}

make_change bounded
cat >"$TMP_DIR/bounded/tasks.md" <<'TASKS'
## 1. Implementation

- [ ] I1 Implement focused parser behavior. [covers=domain.parser.happy,domain.parser.error]
- [ ] I2 Implement focused storage behavior after earlier parser task. Prerequisite: I1 defines input shape. [covers=domain.storage.write]
TASKS
expect_pass bounded

make_change oversized
cat >"$TMP_DIR/oversized/tasks.md" <<'TASKS'
## 1. Implementation

- [ ] I1 Implement parser, storage, auth, and runtime behavior in one task. [covers=domain.parser.happy,domain.storage.write,domain.auth.ticket,domain.runtime.exec]
TASKS
expect_fail oversized "oversized implementation task covers 4 IDs"

make_change integration
cat >"$TMP_DIR/integration/tasks.md" <<'TASKS'
## 2. Verification

- [ ] V1 Run integration proof across parser/storage/auth/runtime boundary and save evidence transcript. [covers=domain.parser.happy,domain.storage.write,domain.auth.ticket,domain.runtime.exec]
TASKS
expect_pass integration

make_change broad-verification-missing-label
cat >"$TMP_DIR/broad-verification-missing-label/tasks.md" <<'TASKS'
## 2. Verification

- [ ] V1 Run broad verification across parser/storage/auth/runtime. [covers=domain.parser.happy,domain.storage.write,domain.auth.ticket,domain.runtime.exec]
TASKS
expect_fail broad-verification-missing-label "broad verification task covers 4 IDs"

make_change out-of-order
cat >"$TMP_DIR/out-of-order/tasks.md" <<'TASKS'
## 1. Implementation

- [ ] I1 Implement runtime adapter after I3 lands. [covers=domain.runtime.exec]
- [ ] I3 Define foundation types. [covers=domain.foundation.types]
TASKS
expect_fail out-of-order "dependency ordering is ambiguous"

make_change ambiguous-dependency
cat >"$TMP_DIR/ambiguous-dependency/tasks.md" <<'TASKS'
## 1. Implementation

- [ ] I1 Implement runtime adapter after foundation work. [covers=domain.runtime.exec]
TASKS
expect_fail ambiguous-dependency "ambiguous foundation dependency"

make_change explicit-prerequisite
cat >"$TMP_DIR/explicit-prerequisite/tasks.md" <<'TASKS'
## 1. Implementation

- [ ] I1 Implement runtime adapter after foundation work. Prerequisite: foundation first is covered by I0 from the parent change. [covers=domain.runtime.exec]
TASKS
expect_pass explicit-prerequisite

echo "task size/order fixtures passed"
