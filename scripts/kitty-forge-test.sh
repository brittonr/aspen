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
TIMEOUT="${ASPEN_TIMEOUT:-30000}"
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

# Source extraction helpers
if [ -f "$SCRIPT_DIR/lib/extraction-helpers.sh" ]; then
    source "$SCRIPT_DIR/lib/extraction-helpers.sh"
fi

# Fallback generate_secret_key if not provided by cluster-common.sh
if ! declare -f generate_secret_key > /dev/null 2>&1; then
    generate_secret_key() {
        local node_id="$1"
        printf '%064x' "$((1000 + node_id))"
    }
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

    # Wait for cluster to fully stabilize
    # This ensures all nodes have received Raft membership through replication
    # and can process non-bootstrap operations (prevents NOT_INITIALIZED errors)
    # CRITICAL: This verifies all nodes have received membership and can accept operations
    printf "  Waiting for cluster stabilization..." >&2
    if wait_for_cluster_stable "$CLI_BIN" "$TICKET" "$TIMEOUT" 60; then
        printf " ${GREEN}done${NC}\n" >&2
    else
        printf " ${RED}FATAL: cluster not stable after 60s${NC}\n" >&2
        printf "${RED}Some nodes may not have received Raft membership. Aborting.${NC}\n" >&2
        exit 1
    fi

    # Wait for forge subsystem to be ready (uses exponential backoff, max 90s)
    # CRITICAL: Fail early if subsystem not ready - tests cannot succeed without it
    printf "  Waiting for Forge subsystem..." >&2
    if wait_for_subsystem "$CLI_BIN" "$TICKET" "$TIMEOUT" forge 90; then
        printf " ${GREEN}done${NC}\n" >&2
    else
        printf " ${RED}FATAL: Forge subsystem not ready after 90s${NC}\n" >&2
        printf "${RED}Tests cannot proceed without Forge subsystem. Aborting.${NC}\n" >&2
        exit 1
    fi

    printf "${GREEN}Cluster ready${NC}\n" >&2
}

# Run CLI command and capture result
LAST_OUTPUT=""
LAST_EXIT_CODE=0

run_cli() {
    local args=("$@")
    local tmpfile
    tmpfile=$(mktemp)
    # More retries with longer delays for distributed system stability
    # Total wait: 2 + 4 + 8 + 16 = 30s max for transient errors
    local max_retries=5
    local retry_delay=2

    for attempt in $(seq 1 $max_retries); do
        set +e
        "$CLI_BIN" --quiet --ticket "$TICKET" --timeout "$TIMEOUT" "${args[@]}" > "$tmpfile" 2>&1
        LAST_EXIT_CODE=$?
        set -e

        LAST_OUTPUT=$(cat "$tmpfile")

        # Check for transient errors that warrant a retry
        # These can occur during cluster formation when not all nodes have
        # fully synchronized their state (Raft membership, subsystem init)
        if [ $LAST_EXIT_CODE -ne 0 ]; then
            if echo "$LAST_OUTPUT" | grep -qE "NOT_INITIALIZED|cluster not initialized|subsystem not ready|FORGE_UNAVAILABLE"; then
                if [ $attempt -lt $max_retries ]; then
                    sleep "$retry_delay"
                    retry_delay=$((retry_delay * 2))
                    # Cap at 16 seconds to avoid excessive waits
                    if [ "$retry_delay" -gt 16 ]; then
                        retry_delay=16
                    fi
                    continue
                fi
            fi
        fi
        break
    done

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

# Run CLI command with retry for transient distributed system issues
run_cli_retry() {
    local max_attempts="${RETRY_ATTEMPTS:-3}"
    local delay="${RETRY_DELAY:-1}"
    local max_delay="${RETRY_MAX_DELAY:-10}"  # Cap backoff at 10 seconds
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if run_cli "$@"; then
            return 0
        fi
        if [ $attempt -lt $max_attempts ]; then
            $VERBOSE && printf "    Retrying in ${delay}s (attempt $attempt/$max_attempts)...\n" >&2
            sleep $delay
            delay=$((delay * 2))  # Exponential backoff
            # Cap the delay at max_delay
            [ $delay -gt $max_delay ] && delay=$max_delay
        fi
        attempt=$((attempt + 1))
    done
    return 1
}

# Run a test with retry support
run_test_retry() {
    local test_name="$1"
    shift
    local cmd=("$@")

    printf "  %-55s " "$test_name"

    local start_time
    start_time=$(date +%s%N)

    if run_cli_retry "${cmd[@]}"; then
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
    printf "${RED}FATAL: Failed to extract repo ID from output${NC}\n" >&2
    printf "${RED}Output was: %s${NC}\n" "$LAST_OUTPUT" >&2
    TESTS_FAILED=$((TESTS_FAILED + 1))
    FAILED_TESTS+=("git init: failed to extract repo ID")
    # Cannot continue without valid repo ID
    printf "${RED}Aborting forge tests - repo ID extraction failed${NC}\n" >&2
    exit 1
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
    printf "${RED}FATAL: Failed to extract blob hash from output${NC}\n" >&2
    printf "${RED}Output was: %s${NC}\n" "$LAST_OUTPUT" >&2
    TESTS_FAILED=$((TESTS_FAILED + 1))
    FAILED_TESTS+=("git store-blob: failed to extract blob hash")
    printf "${RED}Aborting forge tests - blob hash extraction failed${NC}\n" >&2
    exit 1
fi

run_test_expect "git get-blob (retrieve)" "Hello" git get-blob --repo "$REPO_ID" "$BLOB_HASH"

rm -f "$TEMP_FILE"

# Wait for blob replication before tree operations
# Blobs need time to propagate across the cluster before they can be referenced in trees
# This is critical for distributed consistency in multi-node clusters
sleep 5

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# TREE OPERATIONS TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Tree Operations${NC}\n"
fi

# Create tree with the blob (needs blob to be replicated, use retry)
run_test_retry "git create-tree" git create-tree --repo "$REPO_ID" --entry "100644:README.md:$BLOB_HASH"

# Extract tree hash
if [ -n "$LAST_OUTPUT" ]; then
    TREE_HASH=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
fi

if [ -z "$TREE_HASH" ]; then
    printf "${RED}FATAL: Failed to extract tree hash from output${NC}\n" >&2
    printf "${RED}Output was: %s${NC}\n" "$LAST_OUTPUT" >&2
    TESTS_FAILED=$((TESTS_FAILED + 1))
    FAILED_TESTS+=("git create-tree: failed to extract tree hash")
    printf "${RED}Aborting forge tests - tree hash extraction failed${NC}\n" >&2
    exit 1
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
    printf "${RED}FATAL: Failed to extract commit hash from output${NC}\n" >&2
    printf "${RED}Output was: %s${NC}\n" "$LAST_OUTPUT" >&2
    TESTS_FAILED=$((TESTS_FAILED + 1))
    FAILED_TESTS+=("git commit: failed to extract commit hash")
    printf "${RED}Aborting forge tests - commit hash extraction failed${NC}\n" >&2
    exit 1
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

# Push to refs/heads/main (use extended retry for ref operations - need commit replication)
RETRY_ATTEMPTS=5 RETRY_DELAY=2 run_test_retry "git push (set main ref)" git push --repo "$REPO_ID" --ref-name "refs/heads/main" --hash "$COMMIT_HASH"
# Wait for ref to propagate across cluster (increased for distributed consistency)
sleep 5
RETRY_ATTEMPTS=8 RETRY_DELAY=3 run_test_retry "git get-ref (verify)" git get-ref --repo "$REPO_ID" --ref "refs/heads/main"
RETRY_ATTEMPTS=8 RETRY_DELAY=3 run_test_retry "git log (show history)" git log --repo "$REPO_ID" --ref "refs/heads/main" --limit 5

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

run_test_retry "tag create (v1.0.0)" tag create --repo "$REPO_ID" v1.0.0 --target "$COMMIT_HASH"
# Wait for tag to propagate across cluster
sleep 3
RETRY_ATTEMPTS=5 RETRY_DELAY=2 run_test_retry "tag list" tag list --repo "$REPO_ID"
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
    # Use retry for tree creation (needs blob replication)
    run_test_retry "workflow: create second tree" git create-tree --repo "$REPO_ID" --entry "100644:README.md:$BLOB_HASH2"

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
            RETRY_ATTEMPTS=5 RETRY_DELAY=2 run_test_retry "workflow: push update" git push --repo "$REPO_ID" --ref-name "refs/heads/main" --hash "$COMMIT_HASH2"
            # Wait for ref update to propagate across cluster
            sleep 5
            RETRY_ATTEMPTS=8 RETRY_DELAY=3 run_test_retry "workflow: verify log" git log --repo "$REPO_ID" --ref "refs/heads/main" --limit 5
        fi
    fi
fi

rm -f "$TEMP_FILE2"

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# ISSUE (COB) TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Issue Management (COB)${NC}\n"
fi

# Issue state tracking
ISSUE_ID=""

# Create an issue (use --json for reliable ID extraction since human format only shows 8 chars)
run_cli issue create --repo "$REPO_ID" --title "Test Issue ${TEST_PREFIX}" --body "This is a test issue body" --json
if [ $LAST_EXIT_CODE -eq 0 ]; then
    printf "  %-55s ${GREEN}PASS${NC}\n" "issue create"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    printf "  %-55s ${RED}FAIL${NC}\n" "issue create"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    FAILED_TESTS+=("issue create: failed with exit code $LAST_EXIT_CODE")
fi

# Extract issue ID from JSON output
if [ -n "$LAST_OUTPUT" ]; then
    ISSUE_ID=$(extract_issue_id "$LAST_OUTPUT")
fi

if [ -z "$ISSUE_ID" ]; then
    printf "${YELLOW}  Warning: Failed to extract issue ID, skipping issue tests${NC}\n" >&2
    skip_test "issue list" "no issue ID"
    skip_test "issue show" "no issue ID"
    skip_test "issue comment" "no issue ID"
    skip_test "issue close" "no issue ID"
    skip_test "issue reopen" "no issue ID"
else
    run_test_expect "issue list" "Test Issue" issue list --repo "$REPO_ID" --state open --limit 10
    run_test_expect "issue show" "test issue body" issue show --repo "$REPO_ID" "$ISSUE_ID"
    run_test "issue comment" issue comment --repo "$REPO_ID" "$ISSUE_ID" --body "Test comment from CLI"
    run_test "issue close" issue close --repo "$REPO_ID" "$ISSUE_ID" --reason "Testing close"
    run_test_expect "issue list (closed)" "Test Issue" issue list --repo "$REPO_ID" --state closed --limit 10
    run_test "issue reopen" issue reopen --repo "$REPO_ID" "$ISSUE_ID"
    run_test_expect "issue list (reopened)" "Test Issue" issue list --repo "$REPO_ID" --state open --limit 10
fi

if ! $JSON_OUTPUT; then
    printf "\n"
fi

# =============================================================================
# PATCH (COB) TESTS
# =============================================================================
if ! $JSON_OUTPUT; then
    printf "${CYAN}Patch Management (COB)${NC}\n"
fi

# Patch state tracking
PATCH_ID=""

# First create a feature branch for the patch
# Create another commit for the patch head
TEMP_FILE3=$(mktemp)
echo "Feature content for patch" > "$TEMP_FILE3"

run_test "patch setup: store blob" git store-blob --repo "$REPO_ID" "$TEMP_FILE3"

PATCH_BLOB=""
if [ -n "$LAST_OUTPUT" ]; then
    PATCH_BLOB=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
fi

# Use existing COMMIT_HASH as base for the patch
if [ -n "$PATCH_BLOB" ]; then
    run_test "patch setup: create tree" git create-tree --repo "$REPO_ID" --entry "100644:feature.txt:$PATCH_BLOB"

    PATCH_TREE=""
    if [ -n "$LAST_OUTPUT" ]; then
        PATCH_TREE=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
    fi

    if [ -n "$PATCH_TREE" ]; then
        # Use COMMIT_HASH as parent (the base)
        run_test "patch setup: create commit" git commit --repo "$REPO_ID" --tree "$PATCH_TREE" --parent "$COMMIT_HASH" --message "Feature commit for patch"

        PATCH_HEAD=""
        if [ -n "$LAST_OUTPUT" ]; then
            PATCH_HEAD=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
        fi

        if [ -n "$PATCH_HEAD" ] && [ -n "$COMMIT_HASH" ]; then
            # Create patch: base is COMMIT_HASH (main), head is PATCH_HEAD (feature)
            # Use --json for reliable ID extraction since human format only shows 8 chars
            run_cli patch create --repo "$REPO_ID" --title "Test Patch ${TEST_PREFIX}" --description "Test patch description" --base "$COMMIT_HASH" --head "$PATCH_HEAD" --json
            if [ $LAST_EXIT_CODE -eq 0 ]; then
                printf "  %-55s ${GREEN}PASS${NC}\n" "patch create"
                TESTS_PASSED=$((TESTS_PASSED + 1))
            else
                printf "  %-55s ${RED}FAIL${NC}\n" "patch create"
                TESTS_FAILED=$((TESTS_FAILED + 1))
                FAILED_TESTS+=("patch create: failed with exit code $LAST_EXIT_CODE")
            fi

            if [ -n "$LAST_OUTPUT" ]; then
                PATCH_ID=$(extract_patch_id "$LAST_OUTPUT")
            fi
        fi
    fi
fi

rm -f "$TEMP_FILE3"

if [ -z "$PATCH_ID" ]; then
    printf "${YELLOW}  Warning: Failed to create patch, skipping patch tests${NC}\n" >&2
    skip_test "patch list" "no patch ID"
    skip_test "patch show" "no patch ID"
    skip_test "patch approve" "no patch ID"
    skip_test "patch merge" "no patch ID"
    skip_test "patch close" "no patch ID"
else
    run_test_expect "patch list" "Test Patch" patch list --repo "$REPO_ID" --state open --limit 10
    run_test_expect "patch show" "Test patch description" patch show --repo "$REPO_ID" "$PATCH_ID"
    run_test "patch approve" patch approve --repo "$REPO_ID" "$PATCH_ID" --message "LGTM"

    # Create a merge commit for the patch
    run_test "patch merge setup: create merge tree" git create-tree --repo "$REPO_ID" --entry "100644:README.md:$BLOB_HASH" --entry "100644:feature.txt:$PATCH_BLOB"

    MERGE_TREE=""
    if [ -n "$LAST_OUTPUT" ]; then
        MERGE_TREE=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
    fi

    if [ -n "$MERGE_TREE" ] && [ -n "$PATCH_HEAD" ]; then
        # Create merge commit with two parents
        run_test "patch merge setup: create merge commit" git commit --repo "$REPO_ID" --tree "$MERGE_TREE" --parent "$COMMIT_HASH" --parent "$PATCH_HEAD" --message "Merge patch"

        MERGE_COMMIT=""
        if [ -n "$LAST_OUTPUT" ]; then
            MERGE_COMMIT=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
        fi

        if [ -n "$MERGE_COMMIT" ]; then
            run_test "patch merge" patch merge --repo "$REPO_ID" "$PATCH_ID" --merge-commit "$MERGE_COMMIT"
            run_test_expect "patch list (merged)" "Test Patch" patch list --repo "$REPO_ID" --state merged --limit 10
        else
            skip_test "patch merge" "no merge commit"
        fi
    else
        skip_test "patch merge" "no merge tree"
    fi

    # Create another patch to test close
    TEMP_FILE4=$(mktemp)
    echo "Another feature" > "$TEMP_FILE4"
    run_test "patch close setup: store blob" git store-blob --repo "$REPO_ID" "$TEMP_FILE4"

    CLOSE_BLOB=""
    if [ -n "$LAST_OUTPUT" ]; then
        CLOSE_BLOB=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
    fi

    if [ -n "$CLOSE_BLOB" ]; then
        run_cli git create-tree --repo "$REPO_ID" --entry "100644:another.txt:$CLOSE_BLOB"
        CLOSE_TREE=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)

        if [ -n "$CLOSE_TREE" ]; then
            run_cli git commit --repo "$REPO_ID" --tree "$CLOSE_TREE" --parent "$COMMIT_HASH" --message "Another feature"
            CLOSE_HEAD=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)

            if [ -n "$CLOSE_HEAD" ]; then
                run_cli patch create --repo "$REPO_ID" --title "Patch to close" --description "Will be closed" --base "$COMMIT_HASH" --head "$CLOSE_HEAD"
                CLOSE_PATCH=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)

                if [ -n "$CLOSE_PATCH" ]; then
                    run_test "patch close" patch close --repo "$REPO_ID" "$CLOSE_PATCH" --reason "Closing for test"
                    run_test_expect "patch list (closed)" "Patch to close" patch list --repo "$REPO_ID" --state closed --limit 10
                else
                    skip_test "patch close" "failed to create close patch"
                fi
            else
                skip_test "patch close" "failed to create close head"
            fi
        else
            skip_test "patch close" "failed to create close tree"
        fi
    else
        skip_test "patch close" "failed to store close blob"
    fi

    rm -f "$TEMP_FILE4"
fi

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
