#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../../../../.." >/dev/null && pwd)
CHECKER="$REPO_ROOT/scripts/check-openspec-design-verification.py"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

make_change() {
  local name="$1"
  mkdir -p "$TMP_DIR/$name/specs/openspec-governance"
  cat > "$TMP_DIR/$name/specs/openspec-governance/spec.md" <<'SPEC'
## ADDED Requirements

### Requirement: Designs map requirements to verification rails
Design artifacts SHALL include a verification strategy.

ID: openspec-governance.design-verification-strategy

#### Scenario: Design includes mapped verification strategy
ID: openspec-governance.design-verification-strategy.mapped-strategy-present

- **GIVEN** a change has delta specs
- **WHEN** the design gate runs
- **THEN** `design.md` SHALL include `## Verification Strategy`
SPEC
}

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

make_change valid
cat > "$TMP_DIR/valid/design.md" <<'DESIGN'
## Context

Fixture.

## Verification Strategy

- `openspec-governance.design-verification-strategy`: positive check via `scripts/check-openspec-design-verification.py valid`.
DESIGN
run_expect_pass "valid mapped strategy" "$CHECKER" "$TMP_DIR/valid" --repo-root "$REPO_ROOT"

make_change missing_strategy
cat > "$TMP_DIR/missing_strategy/design.md" <<'DESIGN'
## Context

Fixture without strategy.
DESIGN
run_expect_fail "missing strategy" "must include ## Verification Strategy" "$CHECKER" "$TMP_DIR/missing_strategy" --repo-root "$REPO_ROOT"

make_change missing_id
cat > "$TMP_DIR/missing_id/design.md" <<'DESIGN'
## Context

Fixture.

## Verification Strategy

- Positive check via `cargo test`.
DESIGN
run_expect_fail "missing requirement id" "must cite at least one changed requirement or scenario ID" "$CHECKER" "$TMP_DIR/missing_id" --repo-root "$REPO_ROOT"

make_change missing_negative
cat >> "$TMP_DIR/missing_negative/specs/openspec-governance/spec.md" <<'SPEC'

#### Scenario: Invalid input is rejected
ID: openspec-governance.design-verification-strategy.invalid-input-rejected

- **GIVEN** invalid input
- **WHEN** the design gate runs
- **THEN** the gate SHALL reject the design
SPEC
cat > "$TMP_DIR/missing_negative/design.md" <<'DESIGN'
## Context

Fixture.

## Verification Strategy

- `openspec-governance.design-verification-strategy`: positive check via `cargo test`.
DESIGN
run_expect_fail "missing negative mapping" "must name a negative/failure check" "$CHECKER" "$TMP_DIR/missing_negative" --repo-root "$REPO_ROOT"

make_change negative_deferred
cat >> "$TMP_DIR/negative_deferred/specs/openspec-governance/spec.md" <<'SPEC'

#### Scenario: Invalid input is rejected
ID: openspec-governance.design-verification-strategy.invalid-input-rejected

- **GIVEN** invalid input
- **WHEN** the design gate runs
- **THEN** the gate SHALL reject the design
SPEC
cat > "$TMP_DIR/negative_deferred/design.md" <<'DESIGN'
## Context

Fixture.

## Verification Strategy

- `openspec-governance.design-verification-strategy`: positive check via `cargo test`.
- `openspec-governance.design-verification-strategy.invalid-input-rejected`: deferred negative check with rationale: runtime fixture is introduced in the next slice.
DESIGN
run_expect_pass "negative defer rationale" "$CHECKER" "$TMP_DIR/negative_deferred" --repo-root "$REPO_ROOT"
