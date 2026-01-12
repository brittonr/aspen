#!/usr/bin/env bash
# Kitty cluster Forge/Git CLI integration test
#
# This script starts a kitty cluster (or uses an existing one) and runs
# comprehensive tests of all aspen-cli Git/Forge commands.
#
# Usage:
#   nix run .#kitty-forge-test                    # Start cluster and test
#   ./scripts/kitty-forge-test.sh                 # Same as above
#   ./scripts/kitty-forge-test.sh --ticket <ticket>  # Use existing cluster
#   ASPEN_TICKET=<ticket> ./scripts/kitty-forge-test.sh  # Use existing cluster
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

# Forge state tracking (populated during tests)
REPO_ID=""
BLOB_HASH=""
TREE_HASH=""
COMMIT_HASH=""

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

    if [ -n "${ASPEN_CLI_BIN:-}" ] && [ -x "${ASPEN_CLI_BIN}" ]; then
        echo "$ASPEN_CLI_BIN"
        return 0
    fi

    bin=$(command -v aspen-cli 2>/dev/null || echo "")
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

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
        sleep 2
        for pid in "${CLUSTER_PIDS[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                kill -9 "$pid" 2>/dev/null || true
            fi
        done
        if [ -n "$DATA_DIR" ] && [ -d "$DATA_DIR" ]; then
            rm -rf "$DATA_DIR"
        fi
    fi
}
trap cleanup EXIT

# Start a minimal cluster for testing
start_test_cluster() {
    DATA_DIR="/tmp/aspen-forge-test-$$"
    local cookie="forge-test-$$"
    local log_level="${ASPEN_LOG_LEVEL:-warn}"

    # Clean up any lingering processes
    if pgrep -f "aspen-node.*forge-test" >/dev/null 2>&1; then
        printf "  Cleaning up old test processes..." >&2
        pkill -9 -f "aspen-node.*forge-test" 2>/dev/null || true
        sleep 1
        printf " done\n" >&2
    fi

    printf "${BLUE}Starting $NODE_COUNT-node test cluster...${NC}\n" >&2
    mkdir -p "$DATA_DIR"

    for id in $(seq 1 $NODE_COUNT); do
        local node_dir="$DATA_DIR/node$id"
        mkdir -p "$node_dir"
        local secret
        secret=$(generate_secret_key "$id")

        RUST_LOG="$log_level" \
        ASPEN_BLOBS_ENABLED=true \
        ASPEN_DOCS_ENABLED=true \
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
        sleep 3

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
        TEST_RESULTS+=("{\"name\":\"$test_name\",\"status\":\"fail\",\"duration_ms\":$duration_ms}")
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

# Skip a test
skip_test() {
    local test_name="$1"
    local reason="$2"

    printf "  %-55s ${YELLOW}SKIP${NC} (%s)\n" "$test_name" "$reason"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
    TEST_RESULTS+=("{\"name\":\"$test_name\",\"status\":\"skip\",\"reason\":\"$reason\"}")
}

# Generate unique test prefix
TEST_PREFIX="__forge_test_$$_$(date +%s)_"

# =============================================================================
# MAIN
# =============================================================================

CLI_BIN=$(find_cli)
if [ -z "$CLI_BIN" ]; then
    printf "${RED}Error: aspen-cli binary not found${NC}\n" >&2
    printf "Build with: cargo build --release -p aspen-cli\n" >&2
    exit 1
fi

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
    exit 1
fi

# Print header
if ! $JSON_OUTPUT; then
    printf "\n${BLUE}${BOLD}Aspen Forge/Git CLI Integration Test${NC}\n"
    printf "${BLUE}================================================================${NC}\n"
    printf "CLI:     %s\n" "$CLI_BIN"
    printf "Ticket:  %s...%s\n" "${TICKET:0:20}" "${TICKET: -10}"
    printf "Timeout: %s ms\n" "$TIMEOUT"
    printf "\n"
fi

# =============================================================================
# REPOSITORY MANAGEMENT TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Repository Management${NC}\n"
fi

# Create repository and capture ID
run_test "git init (create repo)" git init "${TEST_PREFIX}test-repo" --description "Test repository"

# Extract repo ID from output for subsequent tests
if [ -n "$LAST_OUTPUT" ]; then
    REPO_ID=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
fi

if [ -z "$REPO_ID" ]; then
    printf "${RED}Failed to extract repo ID, using placeholder${NC}\n" >&2
    REPO_ID="0000000000000000000000000000000000000000000000000000000000000000"
fi

run_test_expect "git list (verify repo)" "${TEST_PREFIX}test-repo" git list --limit 10
run_test "git show (repo info)" git show --repo "$REPO_ID"

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# BLOB STORAGE TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Blob Storage${NC}\n"
fi

# Create a test file and store it
TEST_CONTENT="Hello, Forge! Test content from ${TEST_PREFIX}"
TEMP_FILE=$(mktemp)
echo "$TEST_CONTENT" > "$TEMP_FILE"

run_test "git store-blob (store file)" git store-blob --repo "$REPO_ID" "$TEMP_FILE"

# Extract blob hash
if [ -n "$LAST_OUTPUT" ]; then
    BLOB_HASH=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
fi

if [ -z "$BLOB_HASH" ]; then
    printf "${RED}Failed to extract blob hash${NC}\n" >&2
    BLOB_HASH="0000000000000000000000000000000000000000000000000000000000000000"
fi

run_test_expect "git get-blob (retrieve)" "Hello" git get-blob --repo "$REPO_ID" "$BLOB_HASH"

rm -f "$TEMP_FILE"

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# TREE OPERATIONS TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Tree Operations${NC}\n"
fi

# Create tree with the blob
run_test "git create-tree" git create-tree --repo "$REPO_ID" --entry "100644,README.md,$BLOB_HASH"

# Extract tree hash
if [ -n "$LAST_OUTPUT" ]; then
    TREE_HASH=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
fi

if [ -z "$TREE_HASH" ]; then
    printf "${RED}Failed to extract tree hash${NC}\n" >&2
    TREE_HASH="0000000000000000000000000000000000000000000000000000000000000000"
fi

run_test_expect "git get-tree" "README.md" git get-tree --repo "$REPO_ID" "$TREE_HASH"

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# COMMIT OPERATIONS TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Commit Operations${NC}\n"
fi

# Create initial commit
run_test "git commit (initial)" git commit --repo "$REPO_ID" --tree "$TREE_HASH" --message "Initial commit from test"

# Extract commit hash
if [ -n "$LAST_OUTPUT" ]; then
    COMMIT_HASH=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
fi

if [ -z "$COMMIT_HASH" ]; then
    printf "${RED}Failed to extract commit hash${NC}\n" >&2
    COMMIT_HASH="0000000000000000000000000000000000000000000000000000000000000000"
fi

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# REF OPERATIONS TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Ref Operations${NC}\n"
fi

# Push to refs/heads/main
run_test "git push (set main ref)" git push --repo "$REPO_ID" --ref "refs/heads/main" --hash "$COMMIT_HASH"
run_test_expect "git get-ref (verify)" "$COMMIT_HASH" git get-ref --repo "$REPO_ID" --ref "refs/heads/main"
run_test_expect "git log (show history)" "Initial commit" git log --repo "$REPO_ID" --ref "refs/heads/main" --limit 5

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# BRANCH OPERATIONS TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Branch Operations${NC}\n"
fi

run_test "branch create (feature branch)" branch create --repo "$REPO_ID" feature-test --from "$COMMIT_HASH"
run_test_expect "branch list" "feature-test|main" branch list --repo "$REPO_ID"
run_test "branch delete" branch delete --repo "$REPO_ID" feature-test --force

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# TAG OPERATIONS TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Tag Operations${NC}\n"
fi

run_test "tag create (v1.0.0)" tag create --repo "$REPO_ID" v1.0.0 --target "$COMMIT_HASH"
run_test_expect "tag list" "v1.0.0" tag list --repo "$REPO_ID"
run_test "tag delete" tag delete --repo "$REPO_ID" v1.0.0

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# END-TO-END WORKFLOW TEST
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}End-to-End Workflow${NC}\n"
fi

# Create second commit (chained)
TEMP_FILE2=$(mktemp)
echo "Updated content - second commit" > "$TEMP_FILE2"
run_test "workflow: store second blob" git store-blob --repo "$REPO_ID" "$TEMP_FILE2"

BLOB_HASH2=""
if [ -n "$LAST_OUTPUT" ]; then
    BLOB_HASH2=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
fi

if [ -n "$BLOB_HASH2" ]; then
    run_test "workflow: create second tree" git create-tree --repo "$REPO_ID" --entry "100644,README.md,$BLOB_HASH2"

    TREE_HASH2=""
    if [ -n "$LAST_OUTPUT" ]; then
        TREE_HASH2=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
    fi

    if [ -n "$TREE_HASH2" ]; then
        run_test "workflow: create second commit" git commit --repo "$REPO_ID" --tree "$TREE_HASH2" --parent "$COMMIT_HASH" --message "Second commit"

        COMMIT_HASH2=""
        if [ -n "$LAST_OUTPUT" ]; then
            COMMIT_HASH2=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
        fi

        if [ -n "$COMMIT_HASH2" ]; then
            run_test "workflow: push update" git push --repo "$REPO_ID" --ref "refs/heads/main" --hash "$COMMIT_HASH2"
            run_test_expect "workflow: verify log" "Second commit" git log --repo "$REPO_ID" --ref "refs/heads/main" --limit 5
        fi
    fi
fi

rm -f "$TEMP_FILE2"

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# FEDERATION TESTS (SKIPPED - not yet wired)
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Federation Commands${NC}\n"
fi

skip_test "git federate" "federation not wired into RPC layer"
skip_test "git list-federated" "federation not wired into RPC layer"
skip_test "git fetch-remote" "federation not wired into RPC layer"

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# SUMMARY
# =============================================================================
if $JSON_OUTPUT; then
    printf '{"passed":%d,"failed":%d,"skipped":%d,"tests":[%s]}\n' \
        "$TESTS_PASSED" "$TESTS_FAILED" "$TESTS_SKIPPED" \
        "$(IFS=,; echo "${TEST_RESULTS[*]}")"
else
    printf "${BLUE}================================================================${NC}\n"
    printf "${BOLD}Test Summary${NC}\n"
    printf "${BLUE}================================================================${NC}\n"
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
        printf "${GREEN}${BOLD}All Forge/Git tests passed!${NC}\n"
    else
        printf "${RED}${BOLD}Some Forge/Git tests failed.${NC}\n"
    fi
fi

[ "$TESTS_FAILED" -eq 0 ]
