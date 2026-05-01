#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../../../../.." >/dev/null && pwd)
CHECKER="$REPO_ROOT/scripts/check-openspec-proposal-verification.py"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

make_change() {
  local name="$1"
  mkdir -p "$TMP_DIR/$name/specs/example"
  cat >"$TMP_DIR/$name/specs/example/spec.md" <<'SPEC'
## ADDED Requirements

### Requirement: Fixture Capability
Fixture capability SHALL be checked.

ID: example.fixture.traceability

#### Scenario: Happy path
ID: example.fixture.traceability.happy

- **GIVEN** a complete proposal
- **WHEN** the proposal gate runs
- **THEN** it SHALL pass
SPEC
}

make_negative_change() {
  local name="$1"
  mkdir -p "$TMP_DIR/$name/specs/example"
  cat >"$TMP_DIR/$name/specs/example/spec.md" <<'SPEC'
## ADDED Requirements

### Requirement: Fixture Rejection Capability
Fixture rejection capability SHALL reject invalid input.

ID: example.fixture.guard

#### Scenario: Invalid input path
ID: example.fixture.guard.bad-input

- **GIVEN** invalid input
- **WHEN** the proposal gate runs
- **THEN** it SHALL reject the input with an error
SPEC
}

expect_pass() {
  local name="$1"
  "$CHECKER" "$TMP_DIR/$name" --repo-root "$TMP_DIR" >/tmp/proposal-check.out 2>&1 || {
    echo "unexpected failure for $name"
    cat /tmp/proposal-check.out
    return 1
  }
  echo "PASS: $name"
}

expect_fail() {
  local name="$1"
  local expected="$2"
  if "$CHECKER" "$TMP_DIR/$name" --repo-root "$TMP_DIR" >/tmp/proposal-check.out 2>&1; then
    echo "unexpected pass for $name"
    cat /tmp/proposal-check.out
    return 1
  fi
  if ! grep -Fq "$expected" /tmp/proposal-check.out; then
    echo "missing expected failure for $name: $expected"
    cat /tmp/proposal-check.out
    return 1
  fi
  echo "PASS: $name rejected: $expected"
}

make_change complete-mapping
cat >"$TMP_DIR/complete-mapping/proposal.md" <<'PROPOSAL'
## Why

Need traceability.

## Verification Expectations

- `example.fixture.traceability`: positive checker fixture verifies the requirement mapping.
- `example.fixture.traceability.happy`: positive checker fixture verifies the scenario mapping.
PROPOSAL
expect_pass complete-mapping

make_change missing-id
cat >"$TMP_DIR/missing-id/proposal.md" <<'PROPOSAL'
## Why

Need traceability.

## Verification Expectations

- `example.fixture.traceability`: positive checker fixture verifies the requirement mapping.
PROPOSAL
expect_fail missing-id "example.fixture.traceability.happy"

make_negative_change missing-negative
cat >"$TMP_DIR/missing-negative/proposal.md" <<'PROPOSAL'
## Why

Need traceability for rejection behavior.

## Verification Expectations

- `example.fixture.guard`: positive checker fixture verifies the requirement mapping.
- `example.fixture.guard.bad-input`: positive checker fixture verifies the scenario mapping.
PROPOSAL
expect_fail missing-negative "negative-path expectation"

make_change valid-deferral
cat >"$TMP_DIR/valid-deferral/proposal.md" <<'PROPOSAL'
## Why

Need traceability.

## Verification Expectations

- `example.fixture.traceability`: defer to design because exact checker command depends on implementation language.
- `example.fixture.traceability.happy`: defer to design because exact fixture path depends on task decomposition.
PROPOSAL
expect_pass valid-deferral

make_change invalid-deferral
cat >"$TMP_DIR/invalid-deferral/proposal.md" <<'PROPOSAL'
## Why

Need traceability.

## Verification Expectations

- `example.fixture.traceability`: defer to design.
- `example.fixture.traceability.happy`: positive checker fixture verifies the scenario mapping.
PROPOSAL
expect_fail invalid-deferral "defer to design entries must cite a changed requirement/scenario ID and rationale"

echo "proposal verification fixtures passed"
