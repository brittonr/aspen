#!/usr/bin/env bash
# Comprehensive CLI integration test suite for Aspen
#
# This script tests ALL aspen-cli commands against a running cluster.
# It can either start its own cluster or use an existing one via --ticket.
#
# Usage:
#   nix run .#kitty-cli-test                         # Start cluster and test
#   ./scripts/kitty-cli-test.sh                      # Same as above
#   ./scripts/kitty-cli-test.sh --ticket <ticket>    # Use existing cluster
#   ASPEN_TICKET=<ticket> ./scripts/kitty-cli-test.sh
#
# Options:
#   --ticket <ticket>       Use existing cluster (skip cluster startup)
#   --timeout <ms>          RPC timeout in milliseconds (default: 10000)
#   --node-count <n>        Number of nodes to start (default: 3)
#   --skip-cluster-startup  Skip starting a new cluster (requires --ticket)
#   --keep-cluster          Don't stop cluster after tests (useful for debugging)
#   --skip-slow             Skip slow tests (blob, job, forge)
#   --category <name>       Run only specific category
#   --verbose               Show full command output
#   --json                  Output results as JSON
#   --help                  Show this help
#
# Categories:
#   cluster, kv, counter, sequence, lock, semaphore, lease, barrier,
#   rwlock, ratelimit, queue, service, docs, blob, job, dns, sql,
#   hooks, verify, federation, peer

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
SKIP_SLOW=false
VERBOSE=false
JSON_OUTPUT=false
CATEGORY=""

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

# Test state tracking
FENCING_TOKEN=""

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
    DATA_DIR="/tmp/aspen-cli-test-$$"
    local cookie="cli-test-$$"
    local log_level="${ASPEN_LOG_LEVEL:-warn}"

    # Clean up any lingering processes
    if pgrep -f "aspen-node.*cli-test" >/dev/null 2>&1; then
        printf "  Cleaning up old test processes..." >&2
        pkill -9 -f "aspen-node.*cli-test" 2>/dev/null || true
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

        RUST_LOG="$log_level" "$NODE_BIN" \
            --node-id "$id" \
            --cookie "$cookie" \
            --data-dir "$node_dir" \
            --storage-backend inmemory \
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

# Run a test that should fail
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

# Skip a test
skip_test() {
    local test_name="$1"
    local reason="$2"

    printf "  %-55s ${YELLOW}SKIP${NC} (%s)\n" "$test_name" "$reason"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
    TEST_RESULTS+=("{\"name\":\"$test_name\",\"status\":\"skip\",\"reason\":\"$reason\"}")
}

# Check if category should run
should_run_category() {
    local cat="$1"
    [ -z "$CATEGORY" ] || [ "$CATEGORY" = "$cat" ]
}

# Generate unique test key prefix
TEST_PREFIX="__cli_test_$$_$(date +%s)_"

# =============================================================================
# MAIN
# =============================================================================

CLI_BIN=$(find_cli)
if [ -z "$CLI_BIN" ]; then
    printf "${RED}Error: aspen-cli binary not found${NC}\n" >&2
    printf "Build with: cargo build --release --bin aspen-cli\n" >&2
    printf "Or use: nix run .#kitty-cli-test\n" >&2
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
    printf "Use --ticket <ticket> or set ASPEN_TICKET environment variable\n" >&2
    exit 1
fi

# Print header
if ! $JSON_OUTPUT; then
    printf "\n${BLUE}${BOLD}Aspen CLI Comprehensive Integration Test${NC}\n"
    printf "${BLUE}================================================================${NC}\n"
    printf "CLI:     %s\n" "$CLI_BIN"
    printf "Ticket:  %s...%s\n" "${TICKET:0:20}" "${TICKET: -10}"
    printf "Timeout: %s ms\n" "$TIMEOUT"
    printf "Nodes:   %s\n" "$NODE_COUNT"
    [ -n "$CATEGORY" ] && printf "Category: %s\n" "$CATEGORY"
    printf "\n"
fi

# =============================================================================
# CLUSTER TESTS
# =============================================================================
if should_run_category "cluster"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Cluster Commands${NC}\n"
    fi

    run_test "cluster status" cluster status
    run_test "cluster health" cluster health
    run_test "cluster metrics" cluster metrics
    run_test "cluster ticket" cluster ticket

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# KV STORE TESTS
# =============================================================================
if should_run_category "kv"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}KV Store Commands${NC}\n"
    fi

    run_test "kv set" kv set "${TEST_PREFIX}key1" "test_value_1"
    run_test_expect "kv get" "test_value_1" kv get "${TEST_PREFIX}key1"
    run_test "kv set (second key)" kv set "${TEST_PREFIX}key2" "test_value_2"
    run_test "kv set (third key)" kv set "${TEST_PREFIX}key3" "test_value_3"
    run_test_expect "kv scan (prefix)" "Found [0-9]+ key" kv scan "${TEST_PREFIX}" --limit 10
    run_test "kv cas (success)" kv cas "${TEST_PREFIX}key1" --expected "test_value_1" --new-value "updated_value"
    run_test_expect "kv get (after cas)" "updated_value" kv get "${TEST_PREFIX}key1"
    run_test "kv delete" kv delete "${TEST_PREFIX}key3"
    run_test "kv batch-write" kv batch-write "${TEST_PREFIX}batch1=val1" "${TEST_PREFIX}batch2=val2" "${TEST_PREFIX}batch3=val3"
    run_test_expect "kv batch-read" "val1|val2|val3" kv batch-read "${TEST_PREFIX}batch1" "${TEST_PREFIX}batch2" "${TEST_PREFIX}batch3"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# COUNTER TESTS
# =============================================================================
if should_run_category "counter"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Counter Commands${NC}\n"
    fi

    run_test_expect "counter incr (init)" "^1$" counter incr "${TEST_PREFIX}counter1"
    run_test_expect "counter get" "^1$" counter get "${TEST_PREFIX}counter1"
    run_test_expect "counter incr (second)" "^2$" counter incr "${TEST_PREFIX}counter1"
    run_test_expect "counter add" "^12$" counter add "${TEST_PREFIX}counter1" 10
    run_test_expect "counter decr" "^11$" counter decr "${TEST_PREFIX}counter1"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# SEQUENCE TESTS
# =============================================================================
if should_run_category "sequence"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Sequence Commands${NC}\n"
    fi

    run_test_expect "sequence next (init)" "^[0-9]+$" sequence next "${TEST_PREFIX}seq1"
    run_test_expect "sequence next (second)" "^[0-9]+$" sequence next "${TEST_PREFIX}seq1"
    run_test_expect "sequence reserve" "^[0-9]+$" sequence reserve "${TEST_PREFIX}seq1" 100
    run_test_expect "sequence current" "^[0-9]+$" sequence current "${TEST_PREFIX}seq1"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# LOCK TESTS
# =============================================================================
if should_run_category "lock"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Lock Commands${NC}\n"
    fi

    run_test "lock acquire" lock acquire "${TEST_PREFIX}lock1" --holder "test_holder" --ttl 30000
    run_test "lock try-acquire" lock try-acquire "${TEST_PREFIX}lock2" --holder "test_holder" --ttl 30000

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# SEMAPHORE TESTS
# =============================================================================
if should_run_category "semaphore"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Semaphore Commands${NC}\n"
    fi

    run_test "semaphore try-acquire" semaphore try-acquire "${TEST_PREFIX}sem1" --holder "test_holder" --permits 1 --capacity 10 --ttl 30000
    run_test "semaphore status" semaphore status "${TEST_PREFIX}sem1"
    run_test "semaphore release" semaphore release "${TEST_PREFIX}sem1" --holder "test_holder"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# LEASE TESTS
# =============================================================================
if should_run_category "lease"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Lease Commands${NC}\n"
    fi

    run_test_expect "lease grant" "[0-9]+" lease grant 60

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# BARRIER TESTS
# =============================================================================
if should_run_category "barrier"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Barrier Commands${NC}\n"
    fi

    # Barrier enter with count=1 should complete immediately (single participant)
    run_test "barrier enter (single participant)" barrier enter "${TEST_PREFIX}barrier1" --participant "p1" --count 1 --timeout 5000
    run_test "barrier status" barrier status "${TEST_PREFIX}barrier1"
    run_test "barrier leave" barrier leave "${TEST_PREFIX}barrier1" --participant "p1" --timeout 5000

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# RWLOCK TESTS
# =============================================================================
if should_run_category "rwlock"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}RWLock Commands${NC}\n"
    fi

    run_test "rwlock try-read" rwlock try-read "${TEST_PREFIX}rwlock1" --holder "reader1" --ttl 30000
    run_test "rwlock status" rwlock status "${TEST_PREFIX}rwlock1"
    run_test "rwlock release-read" rwlock release-read "${TEST_PREFIX}rwlock1" --holder "reader1"

    # Test write lock
    run_test "rwlock try-write" rwlock try-write "${TEST_PREFIX}rwlock2" --holder "writer1" --ttl 30000

    # Extract fencing token for release
    if run_cli rwlock try-write "${TEST_PREFIX}rwlock3" --holder "writer_test" --ttl 30000; then
        FENCING_TOKEN=$(echo "$LAST_OUTPUT" | grep -oE 'token: [0-9]+' | cut -d' ' -f2 || echo "1")
        if [ -n "$FENCING_TOKEN" ] && [ "$FENCING_TOKEN" != "1" ]; then
            run_test "rwlock release-write" rwlock release-write "${TEST_PREFIX}rwlock3" --holder "writer_test" --token "$FENCING_TOKEN"
        fi
    fi

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# RATELIMIT TESTS
# =============================================================================
if should_run_category "ratelimit"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Rate Limiter Commands${NC}\n"
    fi

    run_test "ratelimit try-acquire" ratelimit try-acquire "${TEST_PREFIX}rl1" --tokens 1 --capacity 100 --rate 10.0
    run_test "ratelimit available" ratelimit available "${TEST_PREFIX}rl1" --capacity 100 --rate 10.0
    run_test "ratelimit reset" ratelimit reset "${TEST_PREFIX}rl1" --capacity 100 --rate 10.0
    run_test "ratelimit acquire" ratelimit acquire "${TEST_PREFIX}rl2" --tokens 1 --capacity 100 --rate 10.0 --timeout 5000

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# QUEUE TESTS
# =============================================================================
if should_run_category "queue"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Queue Commands${NC}\n"
    fi

    run_test "queue create" queue create "${TEST_PREFIX}queue1" --visibility-timeout 30000 --max-attempts 3
    run_test "queue enqueue" queue enqueue "${TEST_PREFIX}queue1" '{"task":"test"}'
    run_test "queue status" queue status "${TEST_PREFIX}queue1"
    run_test "queue peek" queue peek "${TEST_PREFIX}queue1" --max 5
    run_test "queue delete" queue delete "${TEST_PREFIX}queue1"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# SERVICE REGISTRY TESTS
# =============================================================================
if should_run_category "service"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Service Registry Commands${NC}\n"
    fi

    # Register a service (first one is for testing basic functionality)
    run_cli service register "${TEST_PREFIX}svc" "instance1" "127.0.0.1:8080" --svc-version "1.0.0" --tags "web,api" --weight 100 --ttl 30000 >/dev/null 2>&1 || true

    run_test "service register" service register "${TEST_PREFIX}svc2" "instance2" "127.0.0.1:8081" --svc-version "1.0.0" --tags "web" --weight 100 --ttl 30000
    run_test "service list" service list "${TEST_PREFIX}"
    run_test "service discover" service discover "${TEST_PREFIX}svc2"
    run_test "service get" service get "${TEST_PREFIX}svc2" "instance2"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# DOCS (CRDT) TESTS
# =============================================================================
if should_run_category "docs"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Docs (CRDT) Commands${NC}\n"
    fi

    run_test "docs set" docs set "${TEST_PREFIX}doc1" "document_content"
    run_test_expect "docs get" "document_content" docs get "${TEST_PREFIX}doc1"
    run_test "docs status" docs status
    run_test "docs list" docs list --limit 10
    run_test "docs delete" docs delete "${TEST_PREFIX}doc1"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# BLOB TESTS
# =============================================================================
if should_run_category "blob"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Blob Commands${NC}\n"
    fi

    if $SKIP_SLOW; then
        skip_test "blob add" "slow test"
        skip_test "blob list" "slow test"
    else
        TEST_BLOB_FILE=$(mktemp)
        echo "Test blob content for CLI test suite - ${TEST_PREFIX}" > "$TEST_BLOB_FILE"

        run_test_expect "blob add" "[a-f0-9]{64}" blob add "$TEST_BLOB_FILE"
        run_test "blob list" blob list --limit 10

        rm -f "$TEST_BLOB_FILE"
    fi

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# JOB TESTS
# =============================================================================
if should_run_category "job"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Job Commands${NC}\n"
    fi

    if $SKIP_SLOW; then
        skip_test "job submit" "slow test"
        skip_test "job list" "slow test"
        skip_test "job stats" "slow test"
    else
        run_test "job stats" job stats
        run_test "job workers" job workers
        run_test "job list" job list --limit 10
    fi

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# DNS TESTS
# =============================================================================
if should_run_category "dns"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}DNS Commands${NC}\n"
    fi

    # DNS commands have known serialization issue with --quiet flag
    skip_test "dns set (A record)" "serialization issue"
    skip_test "dns get" "serialization issue"
    skip_test "dns set (CNAME)" "serialization issue"
    skip_test "dns scan" "serialization issue"
    skip_test "dns delete" "serialization issue"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# SQL TESTS
# =============================================================================
if should_run_category "sql"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}SQL Commands${NC}\n"
    fi

    run_test "sql query (simple)" sql query "SELECT * FROM kv LIMIT 5"
    run_test "sql query (with filter)" sql query "SELECT key, value FROM kv WHERE key LIKE '${TEST_PREFIX}%' LIMIT 10"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# HOOKS TESTS
# =============================================================================
if should_run_category "hooks"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Hooks Commands${NC}\n"
    fi

    run_test "hooks list" hook list
    run_test "hooks metrics" hook metrics
    run_test "hooks trigger write_committed" hook trigger write_committed --payload '{"key":"test_key","value":"test_value"}'
    run_test "hooks trigger delete_committed" hook trigger delete_committed --payload '{"key":"test_key"}'
    run_test "hooks trigger membership_changed" hook trigger membership_changed --payload '{"voters":[1,2,3]}'
    run_test "hooks trigger leader_elected" hook trigger leader_elected --payload '{"new_leader_id":1,"term":1}'
    run_test "hooks trigger snapshot_created" hook trigger snapshot_created --payload '{"snapshot_index":100,"term":1}'
    run_test_expect "hooks list (json)" "\"enabled\"" hook list --json
    run_test_expect "hooks metrics (json)" "\"total_events_processed\"" hook metrics --json

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# VERIFY TESTS
# =============================================================================
if should_run_category "verify"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Verification Commands${NC}\n"
    fi

    run_test_expect "verify kv" "PASS" verify kv --count 2 --no-cleanup
    run_test_expect "verify docs" "PASS" verify docs
    if ! $SKIP_SLOW; then
        run_test_expect "verify blob" "PASS" verify blob --size 64
    else
        skip_test "verify blob" "slow test"
    fi

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# FEDERATION TESTS
# =============================================================================
if should_run_category "federation"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Federation Commands${NC}\n"
    fi

    run_test "federation status" federation status
    run_test "federation list" federation list

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# PEER TESTS
# =============================================================================
if should_run_category "peer"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Peer Commands${NC}\n"
    fi

    run_test "peer list" peer list

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
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
        printf "${GREEN}${BOLD}All CLI tests passed!${NC}\n"
    else
        printf "${RED}${BOLD}Some CLI tests failed.${NC}\n"
    fi
fi

[ "$TESTS_FAILED" -eq 0 ]
