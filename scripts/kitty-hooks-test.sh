#!/usr/bin/env bash
# Kitty cluster hooks CLI integration test
#
# This script starts a kitty cluster (or uses an existing one) and runs
# comprehensive tests of all aspen-cli hooks commands.
#
# Usage:
#   nix run .#kitty-hooks-test                    # Start cluster and test
#   ./scripts/kitty-hooks-test.sh                 # Same as above
#   ./scripts/kitty-hooks-test.sh --ticket <ticket>  # Use existing cluster
#   ASPEN_TICKET=<ticket> ./scripts/kitty-hooks-test.sh  # Use existing cluster
#
# Options:
#   --ticket <ticket>       Use existing cluster (skip cluster startup)
#   --timeout <ms>          RPC timeout in milliseconds (default: 10000)
#   --node-count <n>        Number of nodes to start (default: 3)
#   --skip-cluster-startup  Skip starting a new cluster (requires --ticket)
#   --keep-cluster          Don't stop cluster after tests (useful for debugging)
#   --verbose               Show full command output
#   --json                  Output results as JSON
#   --help                  Show this help

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
NODE_COUNT="${ASPEN_NODE_COUNT:-3}"
SKIP_CLUSTER_STARTUP=false
KEEP_CLUSTER=false
VERBOSE=false
JSON_OUTPUT=false

# Cluster management
CLUSTER_STARTED=false
CLUSTER_PIDS=()
DATA_DIR=""

# Test tracking
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0
declare -a FAILED_TESTS=()
declare -a TEST_RESULTS=()

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Source shared functions
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

# Find node binary
find_node() {
    local bin=""

    if [ -n "${ASPEN_NODE_BIN:-}" ] && [ -x "${ASPEN_NODE_BIN}" ]; then
        echo "$ASPEN_NODE_BIN"
        return 0
    fi

    bin=$(command -v aspen-node 2>/dev/null || echo "")
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    for dir in "$PROJECT_DIR/target/release" "$PROJECT_DIR/target/debug" "$PROJECT_DIR/result/bin"; do
        if [ -x "$dir/aspen-node" ]; then
            echo "$dir/aspen-node"
            return 0
        fi
    done

    echo ""
}

CLI_BIN=""
NODE_BIN=""

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
        --node-count)
            NODE_COUNT="$2"
            shift 2
            ;;
        --skip-cluster-startup)
            SKIP_CLUSTER_STARTUP=true
            shift
            ;;
        --keep-cluster)
            KEEP_CLUSTER=true
            shift
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

# Cleanup function
cleanup() {
    if $CLUSTER_STARTED && ! $KEEP_CLUSTER; then
        printf "\n${YELLOW}Stopping cluster...${NC}\n" >&2
        for pid in "${CLUSTER_PIDS[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null || true
            fi
        done
        # Wait for graceful shutdown (increased from 1s to allow Iroh cleanup)
        sleep 2
        for pid in "${CLUSTER_PIDS[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                kill -9 "$pid" 2>/dev/null || true
            fi
        done
        if [ -n "$DATA_DIR" ] && [ -d "$DATA_DIR" ]; then
            rm -rf "$DATA_DIR"
        fi
        printf "${GREEN}Cluster stopped${NC}\n" >&2
    elif $KEEP_CLUSTER && $CLUSTER_STARTED; then
        printf "\n${YELLOW}Keeping cluster running as requested${NC}\n" >&2
        printf "Data directory: $DATA_DIR\n" >&2
        printf "Ticket: $TICKET\n" >&2
    fi
}

trap cleanup EXIT INT TERM

# Generate deterministic secret key for a node
generate_secret_key() {
    local node_id="$1"
    printf '%064x' "$((1000 + node_id))"
}

# Start a minimal cluster for testing
start_test_cluster() {
    DATA_DIR="/tmp/aspen-hooks-test-$$"
    local cookie="hooks-test-$$"
    local log_level="${ASPEN_LOG_LEVEL:-warn}"

    # Clean up any lingering aspen-node processes from previous test runs
    # This prevents port/socket conflicts with rapid successive test runs
    if pgrep -f "aspen-node.*hooks-test" >/dev/null 2>&1; then
        printf "  Cleaning up old test processes..." >&2
        pkill -9 -f "aspen-node.*hooks-test" 2>/dev/null || true
        sleep 1
        printf " done\n" >&2
    fi

    printf "${BLUE}Starting $NODE_COUNT-node test cluster...${NC}\n" >&2
    mkdir -p "$DATA_DIR"

    # Start each node
    for id in $(seq 1 $NODE_COUNT); do
        local node_dir="$DATA_DIR/node$id"
        mkdir -p "$node_dir"
        local secret
        secret=$(generate_secret_key "$id")

        RUST_LOG="$log_level" \
        ASPEN_BLOBS_ENABLED=true \
        ASPEN_DOCS_ENABLED=true \
        ASPEN_HOOKS_ENABLED=true \
        "$NODE_BIN" \
            --node-id "$id" \
            --cookie "$cookie" \
            --data-dir "$node_dir" \
            --storage-backend redb \
            --iroh-secret-key "$secret" \
            > "$node_dir/node.log" 2>&1 &
        local pid=$!
        CLUSTER_PIDS+=("$pid")
        printf "  Started node $id (PID: $pid)\n" >&2
    done

    CLUSTER_STARTED=true

    # Wait for node 1 to emit ticket
    printf "  Waiting for cluster ticket..." >&2
    local timeout=30
    local elapsed=0
    local log_file="$DATA_DIR/node1/node.log"

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$log_file" ]; then
            local ticket_found
            ticket_found=$(grep -oE 'aspen[a-z2-7]{50,200}' "$log_file" 2>/dev/null | head -1 || true)
            if [ -n "$ticket_found" ]; then
                TICKET="$ticket_found"
                printf " ${GREEN}done${NC}\n" >&2
                break
            fi
        fi
        printf "." >&2
        sleep 1
        elapsed=$((elapsed + 1))
    done

    if [ -z "$TICKET" ]; then
        printf " ${RED}timeout${NC}\n" >&2
        printf "${RED}Failed to get cluster ticket. Check logs:${NC}\n" >&2
        cat "$log_file" >&2
        exit 1
    fi

    # Initialize cluster
    printf "  Initializing cluster..." >&2
    local attempts=0
    local max_attempts=5
    while [ "$attempts" -lt "$max_attempts" ]; do
        if "$CLI_BIN" --ticket "$TICKET" --timeout "$TIMEOUT" cluster init >/dev/null 2>&1; then
            printf " ${GREEN}done${NC}\n" >&2
            break
        fi
        attempts=$((attempts + 1))
        if [ "$attempts" -lt "$max_attempts" ]; then
            sleep 2
        fi
    done

    if [ "$attempts" -eq "$max_attempts" ]; then
        printf " ${RED}failed${NC}\n" >&2
        exit 1
    fi

    # Add other nodes as learners if multi-node
    if [ "$NODE_COUNT" -gt 1 ]; then
        printf "  Adding nodes as learners..." >&2
        sleep 3  # Wait for gossip discovery

        for id in $(seq 2 $NODE_COUNT); do
            local node_log="$DATA_DIR/node$id/node.log"
            local endpoint_id
            endpoint_id=$(sed 's/\x1b\[[0-9;]*m//g' "$node_log" 2>/dev/null | \
                grep -oE 'endpoint_id=[a-f0-9]{64}' | head -1 | cut -d= -f2 || true)

            if [ -n "$endpoint_id" ]; then
                "$CLI_BIN" --ticket "$TICKET" --timeout "$TIMEOUT" cluster add-learner \
                    --node-id "$id" --addr "$endpoint_id" >/dev/null 2>&1 || true
            fi
        done
        printf " ${GREEN}done${NC}\n" >&2

        # Promote to voters
        printf "  Promoting to voters..." >&2
        local members=""
        for id in $(seq 1 $NODE_COUNT); do
            [ -n "$members" ] && members="$members "
            members="$members$id"
        done
        sleep 2
        "$CLI_BIN" --ticket "$TICKET" --timeout "$TIMEOUT" cluster change-membership $members >/dev/null 2>&1 || true
        printf " ${GREEN}done${NC}\n" >&2
    fi

    # Let cluster stabilize
    sleep 2
    printf "${GREEN}Cluster ready${NC}\n" >&2
}

# Run CLI command and capture result
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
run_test() {
    local test_name="$1"
    shift
    local cmd=("$@")

    printf "  %-55s " "$test_name"

    local start_time
    start_time=$(date +%s%N)

    if run_cli "${cmd[@]}"; then
        local end_time
        end_time=$(date +%s%N)
        local duration_ms=$(( (end_time - start_time) / 1000000 ))

        printf "${GREEN}PASS${NC} (${duration_ms}ms)\n"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        TEST_RESULTS+=("{\"name\":\"$test_name\",\"status\":\"pass\",\"duration_ms\":$duration_ms}")
    else
        local end_time
        end_time=$(date +%s%N)
        local duration_ms=$(( (end_time - start_time) / 1000000 ))

        printf "${RED}FAIL${NC} (${duration_ms}ms)\n"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        FAILED_TESTS+=("$test_name: ${cmd[*]}")
        TEST_RESULTS+=("{\"name\":\"$test_name\",\"status\":\"fail\",\"duration_ms\":$duration_ms,\"error\":\"$LAST_OUTPUT\"}")
    fi
}

# Run a test that expects specific output
run_test_expect() {
    local test_name="$1"
    local expected="$2"
    shift 2
    local cmd=("$@")

    printf "  %-55s " "$test_name"

    local start_time
    start_time=$(date +%s%N)

    if run_cli "${cmd[@]}"; then
        if echo "$LAST_OUTPUT" | grep -qE "$expected"; then
            local end_time
            end_time=$(date +%s%N)
            local duration_ms=$(( (end_time - start_time) / 1000000 ))

            printf "${GREEN}PASS${NC} (${duration_ms}ms)\n"
            TESTS_PASSED=$((TESTS_PASSED + 1))
            TEST_RESULTS+=("{\"name\":\"$test_name\",\"status\":\"pass\",\"duration_ms\":$duration_ms}")
        else
            local end_time
            end_time=$(date +%s%N)
            local duration_ms=$(( (end_time - start_time) / 1000000 ))

            printf "${RED}FAIL${NC} (unexpected output) (${duration_ms}ms)\n"
            TESTS_FAILED=$((TESTS_FAILED + 1))
            FAILED_TESTS+=("$test_name: expected '$expected', got '$LAST_OUTPUT'")
            TEST_RESULTS+=("{\"name\":\"$test_name\",\"status\":\"fail\",\"duration_ms\":$duration_ms}")
        fi
    else
        local end_time
        end_time=$(date +%s%N)
        local duration_ms=$(( (end_time - start_time) / 1000000 ))

        printf "${RED}FAIL${NC} (${duration_ms}ms)\n"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        FAILED_TESTS+=("$test_name: ${cmd[*]}")
        TEST_RESULTS+=("{\"name\":\"$test_name\",\"status\":\"fail\",\"duration_ms\":$duration_ms}")
    fi
}

# Run a test that should fail (e.g., invalid input)
run_test_expect_fail() {
    local test_name="$1"
    shift
    local cmd=("$@")

    printf "  %-55s " "$test_name"

    local start_time
    start_time=$(date +%s%N)

    if ! run_cli "${cmd[@]}"; then
        local end_time
        end_time=$(date +%s%N)
        local duration_ms=$(( (end_time - start_time) / 1000000 ))

        printf "${GREEN}PASS${NC} (expected failure) (${duration_ms}ms)\n"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        TEST_RESULTS+=("{\"name\":\"$test_name\",\"status\":\"pass\",\"duration_ms\":$duration_ms}")
    else
        local end_time
        end_time=$(date +%s%N)
        local duration_ms=$(( (end_time - start_time) / 1000000 ))

        printf "${RED}FAIL${NC} (should have failed) (${duration_ms}ms)\n"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        FAILED_TESTS+=("$test_name: should have failed but succeeded")
        TEST_RESULTS+=("{\"name\":\"$test_name\",\"status\":\"fail\",\"duration_ms\":$duration_ms}")
    fi
}

# =============================================================================
# MAIN
# =============================================================================

# Validate prerequisites
CLI_BIN=$(find_cli)
if [ -z "$CLI_BIN" ]; then
    printf "${RED}Error: aspen-cli binary not found${NC}\n" >&2
    printf "Build with: cargo build --release --bin aspen-cli\n" >&2
    printf "Or use: nix run .#kitty-hooks-test\n" >&2
    exit 1
fi

# Start cluster if needed
if [ -z "$TICKET" ] && ! $SKIP_CLUSTER_STARTUP; then
    NODE_BIN=$(find_node)
    if [ -z "$NODE_BIN" ]; then
        printf "${RED}Error: aspen-node binary not found${NC}\n" >&2
        printf "Build with: cargo build --release --bin aspen-node\n" >&2
        exit 1
    fi
    start_test_cluster
elif [ -z "$TICKET" ]; then
    printf "${RED}Error: No cluster ticket provided and --skip-cluster-startup set${NC}\n" >&2
    printf "Use --ticket <ticket> or set ASPEN_TICKET environment variable\n" >&2
    exit 1
fi

# Print header
if ! $JSON_OUTPUT; then
    printf "\n${BLUE}${BOLD}Aspen Hooks CLI Integration Test${NC}\n"
    printf "${BLUE}======================================================${NC}\n"
    printf "CLI:     %s\n" "$CLI_BIN"
    printf "Ticket:  %s...%s\n" "${TICKET:0:20}" "${TICKET: -10}"
    printf "Timeout: %s ms\n" "$TIMEOUT"
    printf "\n"
fi

# =============================================================================
# HOOKS LIST TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Hook List Commands${NC}\n"
fi

run_test "hooks list (basic)" hook list
run_test_expect "hooks list (json format)" "\"enabled\"" hook list --json
run_test_expect "hooks list (has handlers array)" "\"handlers\"" hook list --json

# =============================================================================
# HOOKS METRICS TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Hook Metrics Commands${NC}\n"
fi

run_test "hooks metrics (basic)" hook metrics
run_test_expect "hooks metrics (json format)" "\"enabled\"" hook metrics --json
run_test_expect "hooks metrics (has total_events)" "\"total_events_processed\"" hook metrics --json
run_test "hooks metrics (with filter - nonexistent)" hook metrics --handler "nonexistent_handler"
run_test_expect "hooks metrics (filtered json)" "\"handlers\"" hook metrics --handler "test_handler" --json

# =============================================================================
# HOOKS TRIGGER TESTS - VALID EVENT TYPES
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Hook Trigger Commands (Valid Event Types)${NC}\n"
fi

# Test all valid event types
run_test "hooks trigger write_committed" \
    hook trigger write_committed --payload '{"key":"test_key","value":"test_value"}'

run_test "hooks trigger delete_committed" \
    hook trigger delete_committed --payload '{"key":"test_key"}'

run_test "hooks trigger membership_changed" \
    hook trigger membership_changed --payload '{"voters":[1,2,3],"learners":[]}'

run_test "hooks trigger leader_elected" \
    hook trigger leader_elected --payload '{"new_leader_id":1,"previous_leader_id":null,"term":1}'

run_test "hooks trigger snapshot_created" \
    hook trigger snapshot_created --payload '{"snapshot_index":100,"term":1,"entry_count":50}'

# =============================================================================
# HOOKS TRIGGER TESTS - PAYLOAD VARIATIONS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Hook Trigger Commands (Payload Variations)${NC}\n"
fi

# Default empty payload
run_test "hooks trigger (default empty payload)" \
    hook trigger write_committed

# Complex nested payload
run_test "hooks trigger (nested payload)" \
    hook trigger write_committed --payload '{"key":"nested/path/key","metadata":{"created_at":"2024-01-01","tags":["test","hooks"]}}'

# Minimal payload
run_test "hooks trigger (minimal payload)" \
    hook trigger leader_elected --payload '{}'

# Large-ish payload (within limits)
run_test "hooks trigger (larger payload)" \
    hook trigger write_committed --payload '{"key":"bulk_test","values":["a","b","c","d","e","f","g","h","i","j"],"count":10}'

# =============================================================================
# HOOKS TRIGGER TESTS - ERROR CASES
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Hook Trigger Commands (Error Cases)${NC}\n"
fi

# Invalid event type should fail
run_test_expect_fail "hooks trigger (invalid event type)" \
    hook trigger invalid_event_type --payload '{}'

# Invalid JSON payload should fail
run_test_expect_fail "hooks trigger (invalid json payload)" \
    hook trigger write_committed --payload 'not valid json'

# Another invalid event type
run_test_expect_fail "hooks trigger (unknown_event)" \
    hook trigger unknown_event --payload '{}'

# =============================================================================
# HOOKS OUTPUT FORMAT TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Hook Output Format Tests${NC}\n"
fi

# Verify human-readable output contains expected text
run_test_expect "hooks list (human readable - enabled/disabled)" "Hook System:" hook list
run_test_expect "hooks metrics (human readable - total events)" "Total Events Processed:" hook metrics

# Verify JSON structure
run_test_expect "hooks list json (has enabled field)" "\"enabled\":" hook list --json
run_test_expect "hooks metrics json (has handlers array)" "\"handlers\":" hook metrics --json

# =============================================================================
# HOOKS COMBINED WORKFLOW TEST
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "\n${CYAN}Hook Combined Workflow Test${NC}\n"
fi

# Simulate a realistic workflow: check list, trigger events, check metrics
run_test "workflow: initial list check" hook list

run_test "workflow: trigger write event" \
    hook trigger write_committed --payload '{"key":"workflow_test_1","value":"data"}'

run_test "workflow: trigger another write" \
    hook trigger write_committed --payload '{"key":"workflow_test_2","value":"more_data"}'

run_test "workflow: trigger delete event" \
    hook trigger delete_committed --payload '{"key":"workflow_test_1"}'

run_test "workflow: check metrics after triggers" hook metrics

run_test "workflow: final list check" hook list

# =============================================================================
# SUMMARY
# =============================================================================
if $JSON_OUTPUT; then
    printf '{"passed":%d,"failed":%d,"skipped":%d,"tests":[%s]}\n' \
        "$TESTS_PASSED" "$TESTS_FAILED" "$TESTS_SKIPPED" \
        "$(IFS=,; echo "${TEST_RESULTS[*]}")"
else
    printf "\n${BLUE}======================================================${NC}\n"
    printf "${BOLD}Test Summary${NC}\n"
    printf "${BLUE}======================================================${NC}\n"
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
        printf "${GREEN}${BOLD}All hooks tests passed!${NC}\n"
    else
        printf "${RED}${BOLD}Some hooks tests failed.${NC}\n"
    fi
fi

# Exit with failure if any tests failed
[ "$TESTS_FAILED" -eq 0 ]
