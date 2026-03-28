#!/usr/bin/env bash
# Check that the workspace compiles under important feature combinations.
#
# Catches: dead code, unused imports, missing Ok() wrappers, type mismatches
# that only surface when specific features are on or off.
#
# Usage:
#   scripts/check-feature-matrix.sh          # run all combos
#   scripts/check-feature-matrix.sh quick    # just the 3 fastest combos

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

FAILED=0
PASSED=0

check_combo() {
    local label="$1"
    local features="$2"

    printf "${YELLOW}[CHECK]${NC} %-40s " "$label"

    local cmd
    if [ -z "$features" ]; then
        cmd="cargo check --all-targets 2>&1"
    else
        cmd="cargo check --features $features --all-targets 2>&1"
    fi

    if output=$(eval "$cmd"); then
        printf "${GREEN}OK${NC}\n"
        PASSED=$((PASSED + 1))
    else
        printf "${RED}FAIL${NC}\n"
        echo "$output" | grep "^error" | head -5
        FAILED=$((FAILED + 1))
    fi
}

echo "Feature matrix check for aspen workspace"
echo "========================================="
echo ""

# Core combos that must always work
check_combo "no features (default)"          ""
check_combo "full"                           "full"
check_combo "forge-full"                     "forge-full"

if [ "${1:-}" != "quick" ]; then
    # Individual major features (catches missing propagation)
    check_combo "ci only"                    "ci"
    check_combo "ci-basic only"              "ci-basic"
    check_combo "forge only"                 "forge"
    check_combo "forge + git-bridge"         "forge,git-bridge"
    check_combo "sql only"                   "sql"
    check_combo "blob only"                  "blob"
    check_combo "secrets only"               "secrets"
    check_combo "automerge only"             "automerge"
    check_combo "hooks only"                 "hooks"
    check_combo "jobs only"                  "jobs"
    check_combo "docs only"                  "docs"
    check_combo "federation only"            "federation"
    check_combo "deploy only"                "deploy"
    check_combo "net only"                   "net"

    # Combos matching real binaries
    check_combo "aspen-node features"        "jobs,docs,blob,hooks,automerge"
    check_combo "ci-full"                    "ci-full"
fi

echo ""
echo "========================================="
echo -e "Results: ${GREEN}${PASSED} passed${NC}, ${RED}${FAILED} failed${NC}"

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi
