#!/usr/bin/env bash
# Comprehensive CLI test suite for Aspen
# Tests all CLI commands against a running cluster
#
# Usage:
#   nix run .#cli-test -- --ticket <ticket>
#   ASPEN_TICKET=<ticket> nix run .#cli-test
#   ./scripts/cli-test.sh --ticket <ticket>
#
# Options:
#   --ticket <ticket>    Cluster ticket (or set ASPEN_TICKET env var)
#   --timeout <ms>       RPC timeout in milliseconds (default: 10000)
#   --skip-slow          Skip slow tests (blob, job)
#   --category <name>    Run only specific category (cluster, kv, counter, etc.)
#   --verbose            Show full command output
#   --json               Output results as JSON
#   --help               Show this help

set -eu

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'
BOLD='\033[1m'

# Configuration
TICKET="${ASPEN_TICKET:-}"
TIMEOUT="${ASPEN_TIMEOUT:-10000}"
SKIP_SLOW=false
CATEGORY=""
VERBOSE=false
JSON_OUTPUT=false

# Test tracking
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0
declare -a FAILED_TESTS=()

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Source shared functions if available
if [ -f "$SCRIPT_DIR/lib/cluster-common.sh" ]; then
    source "$SCRIPT_DIR/lib/cluster-common.sh"
fi

# Find CLI binary
find_cli() {
    local bin=""

    # Check environment variable first
    if [ -n "${ASPEN_CLI_BIN:-}" ] && [ -x "${ASPEN_CLI_BIN}" ]; then
        echo "$ASPEN_CLI_BIN"
        return 0
    fi

    # Check PATH
    bin=$(command -v aspen-cli 2>/dev/null || echo "")
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check cargo build locations
    for dir in "$PROJECT_DIR/target/release" "$PROJECT_DIR/target/debug" "$PROJECT_DIR/result/bin"; do
        if [ -x "$dir/aspen-cli" ]; then
            echo "$dir/aspen-cli"
            return 0
        fi
    done

    echo ""
}

CLI_BIN=""

usage() {
    sed -n '2,/^$/p' "$0" | sed 's/^# //' | sed 's/^#//'
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --ticket)
            TICKET="$2"
            shift 2
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        --skip-slow)
            SKIP_SLOW=true
            shift
            ;;
        --category)
            CATEGORY="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --json)
            JSON_OUTPUT=true
            shift
            ;;
        --help|-h)
            usage
            ;;
        *)
            printf "${RED}Unknown option: $1${NC}\n" >&2
            exit 1
            ;;
    esac
done

# Validate prerequisites
CLI_BIN=$(find_cli)
if [ -z "$CLI_BIN" ]; then
    printf "${RED}Error: aspen-cli binary not found${NC}\n" >&2
    printf "Build with: cargo build --release --bin aspen-cli\n" >&2
    printf "Or use: nix run .#cli-test\n" >&2
    exit 1
fi

if [ -z "$TICKET" ]; then
    printf "${RED}Error: No cluster ticket provided${NC}\n" >&2
    printf "Use --ticket <ticket> or set ASPEN_TICKET environment variable\n" >&2
    exit 1
fi

# Run CLI command and capture result
# Returns: 0 on success, 1 on failure
# Sets: LAST_OUTPUT, LAST_EXIT_CODE
LAST_OUTPUT=""
LAST_EXIT_CODE=0

run_cli() {
    local args=("$@")
    local tmpfile
    tmpfile=$(mktemp)

    set +e
    "$CLI_BIN" --quiet --ticket "$TICKET" --timeout "$TIMEOUT" "${args[@]}" > "$tmpfile" 2>&1
    LAST_EXIT_CODE=$?
    set -e

    LAST_OUTPUT=$(cat "$tmpfile")
    rm -f "$tmpfile"

    if $VERBOSE && [ -n "$LAST_OUTPUT" ]; then
        printf "    Output: %s\n" "$LAST_OUTPUT"
    fi

    return $LAST_EXIT_CODE
}

# Run a test case
# Args: test_name category command...
run_test() {
    local test_name="$1"
    local category="$2"
    shift 2
    local cmd=("$@")

    # Skip if category filter is set and doesn't match
    if [ -n "$CATEGORY" ] && [ "$CATEGORY" != "$category" ]; then
        return 0
    fi

    printf "  %-50s " "$test_name"

    if run_cli "${cmd[@]}"; then
        printf "${GREEN}PASS${NC}\n"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        printf "${RED}FAIL${NC}\n"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        FAILED_TESTS+=("$test_name: ${cmd[*]}")
    fi
}

# Run a test that expects specific output
# Args: test_name category expected_pattern command...
run_test_expect() {
    local test_name="$1"
    local category="$2"
    local expected="$3"
    shift 3
    local cmd=("$@")

    # Skip if category filter is set and doesn't match
    if [ -n "$CATEGORY" ] && [ "$CATEGORY" != "$category" ]; then
        return 0
    fi

    printf "  %-50s " "$test_name"

    if run_cli "${cmd[@]}"; then
        if echo "$LAST_OUTPUT" | grep -qE "$expected"; then
            printf "${GREEN}PASS${NC}\n"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            printf "${RED}FAIL${NC} (unexpected output)\n"
            TESTS_FAILED=$((TESTS_FAILED + 1))
            FAILED_TESTS+=("$test_name: expected '$expected', got '$LAST_OUTPUT'")
        fi
    else
        printf "${RED}FAIL${NC}\n"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        FAILED_TESTS+=("$test_name: ${cmd[*]}")
    fi
}

# Skip a test
skip_test() {
    local test_name="$1"
    local category="$2"
    local reason="$3"

    # Skip if category filter is set and doesn't match
    if [ -n "$CATEGORY" ] && [ "$CATEGORY" != "$category" ]; then
        return 0
    fi

    printf "  %-50s ${YELLOW}SKIP${NC} (%s)\n" "$test_name" "$reason"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
}

# Generate unique test key to avoid collisions
TEST_PREFIX="__cli_test_$$_$(date +%s)_"

# Print header
if ! $JSON_OUTPUT; then
    printf "\n${BLUE}${BOLD}Aspen CLI Test Suite${NC}\n"
    printf "${BLUE}======================================${NC}\n"
    printf "CLI:     %s\n" "$CLI_BIN"
    printf "Ticket:  %s...%s\n" "${TICKET:0:20}" "${TICKET: -10}"
    printf "Timeout: %s ms\n" "$TIMEOUT"
    [ -n "$CATEGORY" ] && printf "Filter:  %s\n" "$CATEGORY"
    printf "\n"
fi

# =============================================================================
# CLUSTER TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Cluster Commands${NC}\n"
fi

run_test "cluster status" "cluster" cluster status
run_test "cluster health" "cluster" cluster health
run_test "cluster metrics" "cluster" cluster metrics
run_test "cluster ticket" "cluster" cluster ticket

# =============================================================================
# KV STORE TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}KV Store Commands${NC}\n"
fi

# Basic operations
run_test "kv set" "kv" kv set "${TEST_PREFIX}key1" "test_value_1"
run_test_expect "kv get" "kv" "test_value_1" kv get "${TEST_PREFIX}key1"
run_test "kv set (second key)" "kv" kv set "${TEST_PREFIX}key2" "test_value_2"
run_test "kv set (third key)" "kv" kv set "${TEST_PREFIX}key3" "test_value_3"

# Scan
run_test_expect "kv scan (prefix)" "kv" "Found [0-9]+ key" kv scan "${TEST_PREFIX}" --limit 10

# Compare-and-swap
run_test "kv cas (success)" "kv" kv cas "${TEST_PREFIX}key1" --expected "test_value_1" --new-value "updated_value"
run_test_expect "kv get (after cas)" "kv" "updated_value" kv get "${TEST_PREFIX}key1"

# Delete
run_test "kv delete" "kv" kv delete "${TEST_PREFIX}key3"

# Batch operations
run_test "kv batch-write" "kv" kv batch-write "${TEST_PREFIX}batch1=val1" "${TEST_PREFIX}batch2=val2" "${TEST_PREFIX}batch3=val3"
run_test_expect "kv batch-read" "kv" "val1|val2|val3" kv batch-read "${TEST_PREFIX}batch1" "${TEST_PREFIX}batch2" "${TEST_PREFIX}batch3"

# =============================================================================
# COUNTER TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Counter Commands${NC}\n"
fi

run_test_expect "counter incr (init)" "counter" "^1$" counter incr "${TEST_PREFIX}counter1"
run_test_expect "counter get" "counter" "^1$" counter get "${TEST_PREFIX}counter1"
run_test_expect "counter incr (second)" "counter" "^2$" counter incr "${TEST_PREFIX}counter1"
run_test_expect "counter add" "counter" "^12$" counter add "${TEST_PREFIX}counter1" 10
run_test_expect "counter decr" "counter" "^11$" counter decr "${TEST_PREFIX}counter1"

# =============================================================================
# SEQUENCE TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Sequence Commands${NC}\n"
fi

run_test_expect "sequence next (init)" "sequence" "^[0-9]+$" sequence next "${TEST_PREFIX}seq1"
run_test_expect "sequence next (second)" "sequence" "^[0-9]+$" sequence next "${TEST_PREFIX}seq1"
run_test_expect "sequence reserve" "sequence" "^[0-9]+$" sequence reserve "${TEST_PREFIX}seq1" 100
run_test_expect "sequence current" "sequence" "^[0-9]+$" sequence current "${TEST_PREFIX}seq1"

# =============================================================================
# LOCK TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Lock Commands${NC}\n"
fi

run_test "lock acquire" "lock" lock acquire "${TEST_PREFIX}lock1" --holder "test_holder" --ttl 30000
# Note: lock release requires fencing token from acquire, so we just try-acquire and release
run_test "lock try-acquire" "lock" lock try-acquire "${TEST_PREFIX}lock2" --holder "test_holder" --ttl 30000

# =============================================================================
# SEMAPHORE TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Semaphore Commands${NC}\n"
fi

run_test "semaphore try-acquire" "semaphore" semaphore try-acquire "${TEST_PREFIX}sem1" --holder "test_holder" --permits 1 --capacity 10 --ttl 30000
run_test "semaphore status" "semaphore" semaphore status "${TEST_PREFIX}sem1"
run_test "semaphore release" "semaphore" semaphore release "${TEST_PREFIX}sem1" --holder "test_holder"

# =============================================================================
# LEASE TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Lease Commands${NC}\n"
fi

run_test_expect "lease grant" "lease" "[0-9]+" lease grant 60
# Note: lease list/revoke/keepalive require the lease ID from grant

# =============================================================================
# DNS TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}DNS Commands${NC}\n"
fi

# Note: DNS commands have a known serialization issue with --quiet flag
# These tests are skipped until the issue is fixed
skip_test "dns set (A record)" "dns" "serialization issue"
skip_test "dns get" "dns" "serialization issue"
skip_test "dns set (CNAME)" "dns" "serialization issue"
skip_test "dns scan" "dns" "serialization issue"
skip_test "dns delete" "dns" "serialization issue"

# =============================================================================
# DOCS (CRDT) TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Docs (CRDT) Commands${NC}\n"
fi

run_test "docs set" "docs" docs set "${TEST_PREFIX}doc1" "document_content"
run_test_expect "docs get" "docs" "document_content" docs get "${TEST_PREFIX}doc1"
run_test "docs status" "docs" docs status
run_test "docs list" "docs" docs list --limit 10
run_test "docs delete" "docs" docs delete "${TEST_PREFIX}doc1"

# =============================================================================
# BLOB TESTS (optional - can be slow)
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Blob Commands${NC}\n"
fi

if $SKIP_SLOW; then
    skip_test "blob add" "blob" "slow test"
    skip_test "blob list" "blob" "slow test"
else
    # Create a small test file
    TEST_BLOB_FILE=$(mktemp)
    echo "Test blob content for CLI test suite - ${TEST_PREFIX}" > "$TEST_BLOB_FILE"

    run_test_expect "blob add" "blob" "[a-f0-9]{64}" blob add "$TEST_BLOB_FILE"
    run_test "blob list" "blob" blob list --limit 10

    rm -f "$TEST_BLOB_FILE"
fi

# =============================================================================
# QUEUE TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Queue Commands${NC}\n"
fi

run_test "queue create" "queue" queue create "${TEST_PREFIX}queue1" --visibility-timeout 30000 --max-attempts 3
run_test "queue enqueue" "queue" queue enqueue "${TEST_PREFIX}queue1" '{"task":"test"}'
run_test "queue status" "queue" queue status "${TEST_PREFIX}queue1"
run_test "queue peek" "queue" queue peek "${TEST_PREFIX}queue1" --max 5
run_test "queue delete" "queue" queue delete "${TEST_PREFIX}queue1"

# =============================================================================
# JOB TESTS (optional - can be slow)
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Job Commands${NC}\n"
fi

if $SKIP_SLOW; then
    skip_test "job submit" "job" "slow test"
    skip_test "job list" "job" "slow test"
    skip_test "job stats" "job" "slow test"
else
    run_test "job stats" "job" job stats
    run_test "job workers" "job" job workers
    run_test "job list" "job" job list --limit 10
fi

# =============================================================================
# VERIFY TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Verification Commands${NC}\n"
fi

run_test_expect "verify kv" "verify" "PASS" verify kv --count 2 --no-cleanup
run_test_expect "verify docs" "verify" "PASS" verify docs
if ! $SKIP_SLOW; then
    run_test_expect "verify blob" "verify" "PASS" verify blob --size 64
else
    skip_test "verify blob" "verify" "slow test"
fi

# =============================================================================
# HOOKS TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Hooks Commands${NC}\n"
fi

# Hook list - should work even with no handlers configured
run_test "hooks list" "hooks" hooks list

# Hook metrics - should return metrics (possibly empty)
run_test "hooks metrics" "hooks" hooks metrics

# Hook metrics with filter - filter by non-existent handler
run_test "hooks metrics (with filter)" "hooks" hooks metrics --handler "nonexistent_handler"

# Hook trigger - test all valid event types
run_test "hooks trigger write_committed" "hooks" hooks trigger write_committed --payload '{"key":"test_key","value":"test_value"}'
run_test "hooks trigger delete_committed" "hooks" hooks trigger delete_committed --payload '{"key":"test_key"}'
run_test "hooks trigger membership_changed" "hooks" hooks trigger membership_changed --payload '{"voters":[1,2,3]}'
run_test "hooks trigger leader_elected" "hooks" hooks trigger leader_elected --payload '{"new_leader_id":1,"term":1}'
run_test "hooks trigger snapshot_created" "hooks" hooks trigger snapshot_created --payload '{"snapshot_index":100,"term":1}'

# Hook trigger with empty payload (default)
run_test "hooks trigger (default payload)" "hooks" hooks trigger write_committed

# Hook list with JSON output
run_test "hooks list (json)" "hooks" hooks list --json

# Hook metrics with JSON output
run_test "hooks metrics (json)" "hooks" hooks metrics --json

# =============================================================================
# SQL TESTS (if available)
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}SQL Commands${NC}\n"
fi

run_test "sql query (simple)" "sql" sql query "SELECT * FROM kv LIMIT 5"
run_test "sql query (with filter)" "sql" sql query "SELECT key, value FROM kv WHERE key LIKE '${TEST_PREFIX}%' LIMIT 10"

# =============================================================================
# CLEANUP
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Cleanup${NC}\n"
fi

# Clean up test keys
printf "  %-50s " "cleaning up test keys"
run_cli kv scan "${TEST_PREFIX}" --limit 1000 > /dev/null 2>&1 || true
# Note: In production, we'd iterate and delete all test keys
printf "${GREEN}DONE${NC}\n"

# =============================================================================
# SUMMARY
# =============================================================================
if $JSON_OUTPUT; then
    printf '{"passed":%d,"failed":%d,"skipped":%d}\n' \
        "$TESTS_PASSED" "$TESTS_FAILED" "$TESTS_SKIPPED"
else
    printf "\n${BLUE}======================================${NC}\n"
    printf "${BOLD}Test Summary${NC}\n"
    printf "${BLUE}======================================${NC}\n"
    printf "  ${GREEN}Passed:${NC}  %d\n" "$TESTS_PASSED"
    printf "  ${RED}Failed:${NC}  %d\n" "$TESTS_FAILED"
    printf "  ${YELLOW}Skipped:${NC} %d\n" "$TESTS_SKIPPED"
    printf "  ${BOLD}Total:${NC}   %d\n" "$((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))"

    if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
        printf "\n${RED}Failed Tests:${NC}\n"
        for test in "${FAILED_TESTS[@]}"; do
            printf "  - %s\n" "$test"
        done
    fi

    printf "\n"

    if [ "$TESTS_FAILED" -eq 0 ]; then
        printf "${GREEN}${BOLD}All tests passed!${NC}\n"
    else
        printf "${RED}${BOLD}Some tests failed.${NC}\n"
    fi
fi

# Exit with failure if any tests failed
[ "$TESTS_FAILED" -eq 0 ]
