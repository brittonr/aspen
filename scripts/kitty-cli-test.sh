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
#   secrets, hooks, verify, forge, federation, peer

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

# Note: extract_fencing_token is now provided by lib/extraction-helpers.sh
# Keeping inline fallback for backwards compatibility

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Source shared functions
if [ -f "$SCRIPT_DIR/lib/cluster-common.sh" ]; then
    source "$SCRIPT_DIR/lib/cluster-common.sh"
fi

# Source extraction helpers (provides extract_fencing_token, extract_receipt_handle, etc.)
if [ -f "$SCRIPT_DIR/lib/extraction-helpers.sh" ]; then
    source "$SCRIPT_DIR/lib/extraction-helpers.sh"
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

        RUST_LOG="$log_level" \
        ASPEN_BLOBS_ENABLED=true \
        ASPEN_DOCS_ENABLED=true \
        ASPEN_DNS_SERVER_ENABLED=true \
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

    # Acquire lock and capture fencing token for subsequent operations
    run_test "lock acquire" lock acquire "${TEST_PREFIX}lock1" --holder "test_holder" --ttl 30000
    LOCK1_FENCING_TOKEN=""
    if [ -n "$LAST_OUTPUT" ]; then
        # Use helper function which tries human format first: "Fencing token: 12345"
        LOCK1_FENCING_TOKEN=$(extract_fencing_token "$LAST_OUTPUT")
    fi

    run_test "lock try-acquire" lock try-acquire "${TEST_PREFIX}lock2" --holder "test_holder" --ttl 30000

    # Renew and release require the fencing token from acquire
    if [ -n "$LOCK1_FENCING_TOKEN" ]; then
        run_test "lock renew" lock renew "${TEST_PREFIX}lock1" --holder "test_holder" --fencing-token "$LOCK1_FENCING_TOKEN" --ttl 60000
        run_test "lock release" lock release "${TEST_PREFIX}lock1" --holder "test_holder" --fencing-token "$LOCK1_FENCING_TOKEN"
    else
        skip_test "lock renew" "no fencing token from acquire"
        skip_test "lock release" "no fencing token from acquire"
    fi

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

    # Grant a lease and capture the ID
    run_test_expect "lease grant" "[0-9]+" lease grant 60

    # Extract lease ID from output for subsequent tests
    LEASE_ID=""
    if [ -n "$LAST_OUTPUT" ]; then
        # Use helper function: "Lease 12345 granted with TTL 60s"
        LEASE_ID=$(extract_lease_id "$LAST_OUTPUT")
    fi

    if [ -n "$LEASE_ID" ]; then
        run_test_expect "lease ttl" "[0-9]+" lease ttl "$LEASE_ID"
        run_test "lease keepalive" lease keepalive "$LEASE_ID"
        run_test "lease list" lease list --limit 10
        run_test "lease revoke" lease revoke "$LEASE_ID"
    else
        skip_test "lease ttl" "no lease ID"
        skip_test "lease keepalive" "no lease ID"
        skip_test "lease list" "no lease ID"
        skip_test "lease revoke" "no lease ID"
    fi

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
        # Use helper function: "Write lock acquired. Fencing token: 12345"
        FENCING_TOKEN=$(extract_fencing_token "$LAST_OUTPUT")
        if [ -n "$FENCING_TOKEN" ]; then
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

    # Create queue
    run_test "queue create" queue create "${TEST_PREFIX}queue1" --visibility-timeout 30000 --max-attempts 3

    # Enqueue single message
    run_test "queue enqueue" queue enqueue "${TEST_PREFIX}queue1" '{"task":"test1"}'

    # Enqueue batch (--items expects a JSON array string)
    run_test "queue enqueue-batch" queue enqueue-batch "${TEST_PREFIX}queue1" --items '[{"payload":"batch1"},{"payload":"batch2"},{"payload":"batch3"}]'

    # Check status
    run_test "queue status" queue status "${TEST_PREFIX}queue1"

    # Peek messages
    run_test "queue peek" queue peek "${TEST_PREFIX}queue1" --max 5

    # Dequeue a message (--visibility is required)
    run_test "queue dequeue" queue dequeue "${TEST_PREFIX}queue1" --consumer "test_consumer" --max 1 --visibility 30000

    # Extract receipt handle for ack/nack/extend tests
    RECEIPT_HANDLE=""
    if [ -n "$LAST_OUTPUT" ]; then
        # Use helper function: "(handle: xxx, attempts: n)" or JSON format
        RECEIPT_HANDLE=$(extract_receipt_handle "$LAST_OUTPUT")
    fi

    # Extend visibility timeout
    if [ -n "$RECEIPT_HANDLE" ]; then
        run_test "queue extend" queue extend "${TEST_PREFIX}queue1" "$RECEIPT_HANDLE" --add 60000
        run_test "queue ack" queue ack "${TEST_PREFIX}queue1" "$RECEIPT_HANDLE"
    else
        skip_test "queue extend" "no receipt handle"
        skip_test "queue ack" "no receipt handle"
    fi

    # Dequeue another for nack test
    run_cli queue dequeue "${TEST_PREFIX}queue1" --consumer "test_consumer" --max 1 --visibility 30000
    RECEIPT_HANDLE2=""
    if [ -n "$LAST_OUTPUT" ]; then
        # Use helper function
        RECEIPT_HANDLE2=$(extract_receipt_handle "$LAST_OUTPUT")
    fi

    if [ -n "$RECEIPT_HANDLE2" ]; then
        run_test "queue nack" queue nack "${TEST_PREFIX}queue1" "$RECEIPT_HANDLE2"
    else
        skip_test "queue nack" "no receipt handle"
    fi

    # DLQ operations
    run_test "queue dlq" queue dlq "${TEST_PREFIX}queue1" --max 10

    # Redrive requires an item_id from DLQ - try to get one
    DLQ_ITEM_ID=""
    if [ -n "$LAST_OUTPUT" ]; then
        # Use helper function: "[item_id] (handle: ...)" or JSON format
        DLQ_ITEM_ID=$(extract_dlq_item_id "$LAST_OUTPUT")
    fi

    if [ -n "$DLQ_ITEM_ID" ]; then
        run_test "queue redrive" queue redrive "${TEST_PREFIX}queue1" "$DLQ_ITEM_ID"
    else
        # Use a placeholder ID if DLQ was empty (command should handle gracefully)
        run_test "queue redrive" queue redrive "${TEST_PREFIX}queue1" 1
    fi

    # Cleanup
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

    # Register first service and capture fencing token
    run_test "service register" service register "${TEST_PREFIX}svc" "instance1" "127.0.0.1:8080" --svc-version "1.0.0" --tags "web,api" --weight 100 --ttl 30000

    SERVICE_TOKEN=""
    if [ -n "$LAST_OUTPUT" ]; then
        SERVICE_TOKEN=$(extract_fencing_token "$LAST_OUTPUT")
    fi

    # Register second service
    run_test "service register (second)" service register "${TEST_PREFIX}svc2" "instance2" "127.0.0.1:8081" --svc-version "1.0.0" --tags "web" --weight 100 --ttl 30000

    # List and discover (no token required)
    run_test "service list" service list "${TEST_PREFIX}"
    run_test "service discover" service discover "${TEST_PREFIX}svc"
    run_test "service get" service get "${TEST_PREFIX}svc" "instance1"

    # Health, heartbeat, update, and deregister require fencing token
    if [ -n "$SERVICE_TOKEN" ]; then
        run_test "service health" service health "${TEST_PREFIX}svc" "instance1" --status healthy --token "$SERVICE_TOKEN"
        run_test "service heartbeat" service heartbeat "${TEST_PREFIX}svc" "instance1" --token "$SERVICE_TOKEN"
        run_test "service update" service update "${TEST_PREFIX}svc" "instance1" --weight 200 --token "$SERVICE_TOKEN"
        run_test "service deregister" service deregister "${TEST_PREFIX}svc" "instance1" --token "$SERVICE_TOKEN"
    else
        skip_test "service health" "no fencing token from register"
        skip_test "service heartbeat" "no fencing token from register"
        skip_test "service update" "no fencing token from register"
        skip_test "service deregister" "no fencing token from register"
    fi

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
        skip_test "blob get" "slow test"
        skip_test "blob has" "slow test"
        skip_test "blob status" "slow test"
        skip_test "blob protect" "slow test"
        skip_test "blob unprotect" "slow test"
        skip_test "blob delete" "slow test"
    else
        TEST_BLOB_FILE=$(mktemp)
        echo "Test blob content for CLI test suite - ${TEST_PREFIX}" > "$TEST_BLOB_FILE"

        # Add blob and capture hash
        run_test_expect "blob add" "[a-f0-9]{64}" blob add "$TEST_BLOB_FILE"

        # Extract blob hash for subsequent operations
        BLOB_HASH=""
        if [ -n "$LAST_OUTPUT" ]; then
            BLOB_HASH=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
        fi

        # List blobs
        run_test "blob list" blob list --limit 10

        if [ -n "$BLOB_HASH" ]; then
            # Check if blob exists
            run_test "blob has" blob has "$BLOB_HASH"

            # Get blob content
            BLOB_OUTPUT_FILE=$(mktemp)
            run_test "blob get" blob get "$BLOB_HASH" --output "$BLOB_OUTPUT_FILE"
            rm -f "$BLOB_OUTPUT_FILE"

            # Get blob status
            run_test "blob status" blob status "$BLOB_HASH"

            # Protect blob from garbage collection
            run_test "blob protect" blob protect "$BLOB_HASH" --tag "test-protection"

            # Unprotect blob
            run_test "blob unprotect" blob unprotect "test-protection"

            # Delete blob (cleanup)
            run_test "blob delete" blob delete "$BLOB_HASH"
        else
            skip_test "blob has" "no blob hash"
            skip_test "blob get" "no blob hash"
            skip_test "blob status" "no blob hash"
            skip_test "blob protect" "no blob hash"
            skip_test "blob unprotect" "no blob hash"
            skip_test "blob delete" "no blob hash"
        fi

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
        skip_test "job get" "slow test"
        skip_test "job list" "slow test"
        skip_test "job stats" "slow test"
        skip_test "job status" "slow test"
        skip_test "job cancel" "slow test"
    else
        # Get stats first
        run_test "job stats" job stats
        run_test "job workers" job workers
        run_test "job list" job list --limit 10

        # Submit a test job (positional args: job_type payload, then --priority)
        run_test "job submit" job submit "test" '{"test":"data"}' --priority 5

        # Extract job ID from output
        JOB_ID=""
        if [ -n "$LAST_OUTPUT" ]; then
            # Use helper function: "Job submitted: uuid-with-dashes"
            JOB_ID=$(extract_job_id "$LAST_OUTPUT")
        fi

        if [ -n "$JOB_ID" ]; then
            # Get job details
            run_test "job get" job get "$JOB_ID"

            # Check job status
            run_test "job status" job status "$JOB_ID"

            # Cancel the job
            run_test "job cancel" job cancel "$JOB_ID"
        else
            skip_test "job get" "no job ID"
            skip_test "job status" "no job ID"
            skip_test "job cancel" "no job ID"
        fi
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

    # Test A record operations
    run_test "dns set (A record)" dns set "${TEST_PREFIX}dns.example.com" A 192.168.1.100 --ttl 300
    run_test_expect "dns get (A record)" "192.168.1.100" dns get "${TEST_PREFIX}dns.example.com" A

    # Test CNAME record
    run_test "dns set (CNAME)" dns set "${TEST_PREFIX}alias.example.com" CNAME "${TEST_PREFIX}dns.example.com" --ttl 300
    run_test_expect "dns get (CNAME)" "${TEST_PREFIX}dns.example.com" dns get "${TEST_PREFIX}alias.example.com" CNAME

    # Test TXT record
    run_test "dns set (TXT)" dns set "${TEST_PREFIX}txt.example.com" TXT "test=value" --ttl 300
    run_test_expect "dns get (TXT)" "test=value" dns get "${TEST_PREFIX}txt.example.com" TXT

    # Test scan for DNS records
    run_test_expect "dns scan" "Found [0-9]+ record" dns scan "${TEST_PREFIX}" --limit 10

    # Test resolve (with potential wildcard)
    run_test "dns resolve" dns resolve "${TEST_PREFIX}dns.example.com" A

    # Test get-all for a domain
    run_test "dns get-all" dns get-all "${TEST_PREFIX}dns.example.com"

    # Test delete
    run_test "dns delete" dns delete "${TEST_PREFIX}txt.example.com" TXT

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
# SECRETS TESTS
# =============================================================================
if should_run_category "secrets"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Secrets Commands${NC}\n"
    fi

    # KV v2 Engine - Basic Operations
    run_test "secrets kv put" \
        secrets kv put "${TEST_PREFIX}secrets/config" '{"username":"admin","password":"secret123"}'

    run_test_expect "secrets kv get" "admin" \
        secrets kv get "${TEST_PREFIX}secrets/config"

    run_test "secrets kv list" \
        secrets kv list "${TEST_PREFIX}secrets/"

    run_test_expect "secrets kv metadata" "version" \
        secrets kv metadata "${TEST_PREFIX}secrets/config"

    # Create version 2 for delete/undelete test
    run_test "secrets kv put (v2)" \
        secrets kv put "${TEST_PREFIX}secrets/config" '{"username":"admin","password":"newsecret456"}'

    run_test "secrets kv delete" \
        secrets kv delete "${TEST_PREFIX}secrets/config" --versions 1

    run_test "secrets kv undelete" \
        secrets kv undelete "${TEST_PREFIX}secrets/config" --versions 1

    # Transit Engine - Encryption Operations
    run_test "secrets transit create-key" \
        secrets transit create-key "${TEST_PREFIX}cli-key" --key-type aes256-gcm

    run_test_expect "secrets transit encrypt" "aspen:v" \
        secrets transit encrypt "${TEST_PREFIX}cli-key" "test data for encryption"

    # Capture ciphertext for decrypt test
    run_cli secrets transit encrypt "${TEST_PREFIX}cli-key" "decrypt roundtrip test"
    SECRETS_CT=$(echo "$LAST_OUTPUT" | grep -oE 'aspen:v[0-9]+:[A-Za-z0-9+/=]+' | head -1 || true)

    if [ -n "$SECRETS_CT" ]; then
        run_test_expect "secrets transit decrypt" "decrypt roundtrip test" \
            secrets transit decrypt "${TEST_PREFIX}cli-key" "$SECRETS_CT"
    else
        skip_test "secrets transit decrypt" "no ciphertext captured"
    fi

    run_test_expect "secrets transit list-keys" "${TEST_PREFIX}cli-key" \
        secrets transit list-keys

    run_test "secrets transit rotate-key" \
        secrets transit rotate-key "${TEST_PREFIX}cli-key"

    run_test_expect "secrets transit datakey" "Ciphertext" \
        secrets transit datakey "${TEST_PREFIX}cli-key" --key-type wrapped

    # PKI Engine - Certificate Operations
    run_test_expect "secrets pki generate-root" "BEGIN CERTIFICATE" \
        secrets pki generate-root "CLI Test Root CA" --ttl-days 30

    run_test "secrets pki create-role" \
        secrets pki create-role "${TEST_PREFIX}cli-role" \
            --allowed-domains test.local \
            --allow-bare-domains \
            --max-ttl-days 7

    run_test_expect "secrets pki issue" "BEGIN CERTIFICATE" \
        secrets pki issue "${TEST_PREFIX}cli-role" "test.local" --ttl-days 1

    run_test_expect "secrets pki list-roles" "${TEST_PREFIX}cli-role" \
        secrets pki list-roles

    run_test "secrets pki list-certs" \
        secrets pki list-certs

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
# FORGE TESTS (Git-on-Aspen)
# =============================================================================
if should_run_category "forge"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Forge Commands (Git-on-Aspen)${NC}\n"
    fi

    if $SKIP_SLOW; then
        skip_test "forge (all)" "slow test"
    else
        # Forge state tracking
        FORGE_REPO_ID=""
        FORGE_BLOB_HASH=""
        FORGE_TREE_HASH=""
        FORGE_COMMIT_HASH=""

        # Repository management
        run_test "git init (create repo)" git init "${TEST_PREFIX}forge-repo" --description "Test repository"

        if [ -n "$LAST_OUTPUT" ]; then
            FORGE_REPO_ID=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
        fi

        if [ -n "$FORGE_REPO_ID" ]; then
            run_test_expect "git list (verify repo)" "${TEST_PREFIX}forge-repo" git list --limit 10
            run_test "git show (repo info)" git show --repo "$FORGE_REPO_ID"

            # Blob storage
            TEMP_BLOB_FILE=$(mktemp)
            echo "Test content for Forge blob ${TEST_PREFIX}" > "$TEMP_BLOB_FILE"
            run_test "git store-blob" git store-blob --repo "$FORGE_REPO_ID" "$TEMP_BLOB_FILE"

            if [ -n "$LAST_OUTPUT" ]; then
                FORGE_BLOB_HASH=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
            fi

            if [ -n "$FORGE_BLOB_HASH" ]; then
                run_test_expect "git get-blob" "Test content" git get-blob --repo "$FORGE_REPO_ID" "$FORGE_BLOB_HASH"

                # Tree operations
                run_test "git create-tree" git create-tree --repo "$FORGE_REPO_ID" --entry "100644:README.md:$FORGE_BLOB_HASH"

                if [ -n "$LAST_OUTPUT" ]; then
                    FORGE_TREE_HASH=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
                fi

                if [ -n "$FORGE_TREE_HASH" ]; then
                    run_test_expect "git get-tree" "README.md" git get-tree --repo "$FORGE_REPO_ID" "$FORGE_TREE_HASH"

                    # Commit operations
                    run_test "git commit" git commit --repo "$FORGE_REPO_ID" --tree "$FORGE_TREE_HASH" --message "Initial commit from CLI test"

                    if [ -n "$LAST_OUTPUT" ]; then
                        FORGE_COMMIT_HASH=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
                    fi

                    if [ -n "$FORGE_COMMIT_HASH" ]; then
                        # Ref operations
                        run_test "git push (set ref)" git push --repo "$FORGE_REPO_ID" --ref-name "refs/heads/main" --hash "$FORGE_COMMIT_HASH"
                        run_test_expect "git get-ref" "$FORGE_COMMIT_HASH" git get-ref --repo "$FORGE_REPO_ID" --ref "refs/heads/main"
                        run_test_expect "git log" "Initial commit" git log --repo "$FORGE_REPO_ID" --ref "refs/heads/main" --limit 5

                        # Issue COB (if enabled)
                        run_test "issue create" issue create --repo "$FORGE_REPO_ID" --title "Test Issue ${TEST_PREFIX}" --body "Test issue body"
                        FORGE_ISSUE_ID=""
                        if [ -n "$LAST_OUTPUT" ]; then
                            FORGE_ISSUE_ID=$(echo "$LAST_OUTPUT" | grep -oE '[a-f0-9]{64}' | head -1 || true)
                        fi
                        if [ -n "$FORGE_ISSUE_ID" ]; then
                            run_test_expect "issue list" "Test Issue" issue list --repo "$FORGE_REPO_ID" --state open --limit 10
                            run_test "issue close" issue close --repo "$FORGE_REPO_ID" "$FORGE_ISSUE_ID"
                        fi
                    else
                        skip_test "git push" "no commit hash"
                        skip_test "git get-ref" "no commit hash"
                        skip_test "git log" "no commit hash"
                        skip_test "issue create" "no commit hash"
                    fi
                else
                    skip_test "git get-tree" "no tree hash"
                    skip_test "git commit" "no tree hash"
                    skip_test "git push" "no tree hash"
                    skip_test "git get-ref" "no tree hash"
                    skip_test "git log" "no tree hash"
                    skip_test "issue create" "no tree hash"
                fi
            else
                skip_test "git get-blob" "no blob hash"
                skip_test "git create-tree" "no blob hash"
                skip_test "git get-tree" "no blob hash"
                skip_test "git commit" "no blob hash"
                skip_test "git push" "no blob hash"
                skip_test "git get-ref" "no blob hash"
                skip_test "git log" "no blob hash"
                skip_test "issue create" "no blob hash"
            fi

            rm -f "$TEMP_BLOB_FILE"
        else
            skip_test "git list" "no repo ID"
            skip_test "git show" "no repo ID"
            skip_test "git store-blob" "no repo ID"
            skip_test "git get-blob" "no repo ID"
            skip_test "git create-tree" "no repo ID"
            skip_test "git get-tree" "no repo ID"
            skip_test "git commit" "no repo ID"
            skip_test "git push" "no repo ID"
            skip_test "git get-ref" "no repo ID"
            skip_test "git log" "no repo ID"
            skip_test "issue create" "no repo ID"
        fi
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

    # Federation commands are stubs - server returns "Federation not configured"
    # Skip until federation infrastructure is wired into RPC layer
    printf "  %-50s ${YELLOW}SKIP${NC} (not yet implemented)\n" "federation status"
    printf "  %-50s ${YELLOW}SKIP${NC} (not yet implemented)\n" "federation list"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 2))

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

    # Peer list requires peer_manager which is not initialized in test cluster
    # Skip until peer sync infrastructure is enabled in test setup
    printf "  %-50s ${YELLOW}SKIP${NC} (not yet implemented)\n" "peer list"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 1))

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
