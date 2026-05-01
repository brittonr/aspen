#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../../../../.." >/dev/null && pwd)
CHECKER="$REPO_ROOT/scripts/check-openspec-drain-verification.py"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

make_change() {
  local name="$1"
  mkdir -p "$TMP_DIR/$name"
  cat > "$TMP_DIR/$name/tasks.md" <<'TASKS'
## 1. Implementation

- [x] I1 Implement thing

## 2. Verification

- [x] V1 Verify thing
TASKS
}

write_verification() {
  local name="$1"
  local changed="$2"
  local matrix="$3"
  cat > "$TMP_DIR/$name/verification.md" <<EOF2
# Verification Evidence

## Implementation Evidence

- Changed file: \`$changed\`

## Task Coverage

- [x] I1 Implement thing
  - Evidence: \`$changed\`
- [x] V1 Verify thing
  - Evidence: \`$changed\`

$matrix
EOF2
}

full_matrix='## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | cargo build | pass | evidence/build.txt | full workspace build | n/a |
| test | cargo nextest run | pass | evidence/nextest.txt | full workspace tests | n/a |
| format | nix fmt | pass | evidence/fmt.txt | full workspace format | n/a |'

scoped_matrix='## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | python3 -m py_compile scripts/check.py | scoped | evidence/build.txt | checker-only source changed | cargo build before release |
| test | fixtures/run.sh | scoped | evidence/fixtures.txt | fixture covers checker behavior | cargo nextest run before release |
| format | git diff --check | scoped | evidence/diff-check.txt | whitespace-only format rail for scripts | nix fmt before release |'

doc_matrix='## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | n/a | doc-only | evidence/doc.txt | only markdown/OpenSpec artifacts changed | n/a |
| test | n/a | doc-only | evidence/doc.txt | only markdown/OpenSpec artifacts changed | n/a |
| format | n/a | doc-only | evidence/doc.txt | only markdown/OpenSpec artifacts changed | n/a |'

incomplete_matrix='## Drain Verification Matrix

| Rail | Command | Status | Artifact | Scope rationale | Next best check |
| --- | --- | --- | --- | --- | --- |
| build | cargo build | pass | evidence/build.txt | full workspace build | n/a |'

run_expect_pass() {
  local name="$1"
  shift
  if "$@" > "$TMP_DIR/$name.out" 2>&1; then
    echo "PASS: $name"
    sed 's/^/  /' "$TMP_DIR/$name.out"
  else
    echo "FAIL: expected pass for $name"
    sed 's/^/  /' "$TMP_DIR/$name.out"
    exit 1
  fi
}

run_expect_fail() {
  local name="$1"
  local needle="$2"
  shift 2
  if "$@" > "$TMP_DIR/$name.out" 2>&1; then
    echo "FAIL: expected failure for $name"
    sed 's/^/  /' "$TMP_DIR/$name.out"
    exit 1
  fi
  if grep -Fq "$needle" "$TMP_DIR/$name.out"; then
    echo "PASS: $name"
    sed 's/^/  /' "$TMP_DIR/$name.out"
  else
    echo "FAIL: $name did not report expected text: $needle"
    sed 's/^/  /' "$TMP_DIR/$name.out"
    exit 1
  fi
}

make_change full_cycle
write_verification full_cycle "scripts/check.py" "$full_matrix"
run_expect_pass "full cycle" "$CHECKER" "$TMP_DIR/full_cycle" --repo-root "$REPO_ROOT"

make_change scoped
write_verification scoped "scripts/check.py" "$scoped_matrix"
run_expect_pass "scoped alternative" "$CHECKER" "$TMP_DIR/scoped" --repo-root "$REPO_ROOT"

make_change doc_only
write_verification doc_only "openspec/changes/example/design.md" "$doc_matrix"
run_expect_pass "doc only bypass" "$CHECKER" "$TMP_DIR/doc_only" --repo-root "$REPO_ROOT"

make_change missing_cycle
write_verification missing_cycle "scripts/check.py" ""
run_expect_fail "missing matrix" "must include ## Drain Verification Matrix" "$CHECKER" "$TMP_DIR/missing_cycle" --repo-root "$REPO_ROOT"

make_change final_before_matrix
write_verification final_before_matrix "scripts/check.py" "$incomplete_matrix"
run_expect_fail "final before complete matrix" "missing required rails" "$CHECKER" "$TMP_DIR/final_before_matrix" --repo-root "$REPO_ROOT"
