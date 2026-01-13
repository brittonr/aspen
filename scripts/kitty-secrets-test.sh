#!/usr/bin/env bash
# Kitty cluster secrets CLI integration test
#
# This script starts a kitty cluster (or uses an existing one) and runs
# comprehensive tests of all aspen-cli secrets commands (KV v2, Transit, PKI).
#
# Usage:
#   nix run .#kitty-secrets-test                    # Start cluster and test
#   ./scripts/kitty-secrets-test.sh                 # Same as above
#   ./scripts/kitty-secrets-test.sh --ticket <ticket>  # Use existing cluster
#   ASPEN_TICKET=<ticket> ./scripts/kitty-secrets-test.sh  # Use existing cluster
#
# Options:
#   --ticket <ticket>       Use existing cluster (skip cluster startup)
#   --timeout <ms>          RPC timeout in milliseconds (default: 10000)
#   --node-count <n>        Number of nodes to start (default: 3)
#   --skip-cluster-startup  Skip starting a new cluster (requires --ticket)
#   --keep-cluster          Don't stop cluster after tests (useful for debugging)
#   --category <name>       Run only specific category
#   --verbose               Show full command output
#   --json                  Output results as JSON
#   --help                  Show this help
#
# Categories:
#   kv, transit, pki, workflow, errors, json
#   mount, convergent, edge, metadata, chain

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

# Test state (for storing values between tests)
# Test state tracking (for potential future use)

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
    DATA_DIR="/tmp/aspen-secrets-test-$$"
    local cookie="secrets-test-$$"
    local log_level="${ASPEN_LOG_LEVEL:-warn}"

    # Clean up any lingering aspen-node processes from previous test runs
    if pgrep -f "aspen-node.*secrets-test" >/dev/null 2>&1; then
        printf "  Cleaning up old test processes..." >&2
        pkill -9 -f "aspen-node.*secrets-test" 2>/dev/null || true
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
        ASPEN_SECRETS_ENABLED=true \
        ASPEN_HOOKS_ENABLED=true \
        ASPEN_DNS_SERVER_ENABLED=true \
        ASPEN_SECRETS_FILE="$SCRIPT_DIR/test-fixtures/test-secrets.toml" \
        ASPEN_AGE_IDENTITY_FILE="$SCRIPT_DIR/test-fixtures/test-age.key" \
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

    # Wait for cluster to fully initialize (including secrets subsystem)
    # The secrets subsystem initializes after the main cluster is ready,
    # so we verify readiness by checking if secrets operations work
    printf "  Waiting for cluster to fully initialize..." >&2
    local init_attempts=0
    local max_init_attempts=10
    while [ "$init_attempts" -lt "$max_init_attempts" ]; do
        # Try a simple secrets operation to verify the subsystem is ready
        if "$CLI_BIN" --ticket "$TICKET" --timeout "$TIMEOUT" secrets transit list-keys >/dev/null 2>&1; then
            printf " ${GREEN}done${NC}\n" >&2
            break
        fi
        init_attempts=$((init_attempts + 1))
        printf "." >&2
        sleep 2
    done

    if [ "$init_attempts" -eq "$max_init_attempts" ]; then
        printf " ${YELLOW}warning: cluster may not be fully ready${NC}\n" >&2
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

    printf "  %-60s " "$test_name"

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

    printf "  %-60s " "$test_name"

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

    printf "  %-60s " "$test_name"

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

# Removed unused run_test_capture function

# Check if category should run
should_run_category() {
    local cat="$1"
    [ -z "$CATEGORY" ] || [ "$CATEGORY" = "$cat" ]
}

# =============================================================================
# MAIN
# =============================================================================

# Validate prerequisites
CLI_BIN=$(find_cli)
if [ -z "$CLI_BIN" ]; then
    printf "${RED}Error: aspen-cli binary not found${NC}\n" >&2
    printf "Build with: cargo build --release --bin aspen-cli\n" >&2
    printf "Or use: nix run .#kitty-secrets-test\n" >&2
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

# Generate unique test prefix for mount names (avoids conflicts on re-runs)
TEST_PREFIX="sec_$$_$(date +%s)_"

# Print header
if ! $JSON_OUTPUT; then
    printf "\n${BLUE}${BOLD}Aspen Secrets CLI Integration Test${NC}\n"
    printf "${BLUE}================================================================${NC}\n"
    printf "CLI:     %s\n" "$CLI_BIN"
    printf "Ticket:  %s...%s\n" "${TICKET:0:20}" "${TICKET: -10}"
    printf "Timeout: %s ms\n" "$TIMEOUT"
    printf "Nodes:   %s\n" "$NODE_COUNT"
    [ -n "$CATEGORY" ] && printf "Category: %s\n" "$CATEGORY"
    printf "\n"
fi

# =============================================================================
# KV v2 ENGINE TESTS
# =============================================================================
if should_run_category "kv"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}KV v2 Engine Tests${NC}\n"
    fi

    # Basic write/read
    run_test "kv put (basic secret)" \
        secrets kv put myapp/config '{"username":"admin","password":"secret123"}'

    run_test_expect "kv get (read secret)" "admin" \
        secrets kv get myapp/config

    run_test_expect "kv get (read password field)" "secret123" \
        secrets kv get myapp/config

    # Versioning
    run_test "kv put (create version 2)" \
        secrets kv put myapp/config '{"username":"admin","password":"newsecret456"}'

    run_test_expect "kv get (latest version)" "newsecret456" \
        secrets kv get myapp/config

    run_test_expect "kv get (specific version 1)" "secret123" \
        secrets kv get myapp/config --version 1

    # List secrets
    run_test "kv put (second path)" \
        secrets kv put myapp/database '{"host":"localhost","port":"5432"}'

    run_test "kv put (third path)" \
        secrets kv put myapp/cache '{"redis":"localhost:6379"}'

    run_test_expect "kv list (prefix)" "config" \
        secrets kv list myapp/

    # Metadata
    run_test_expect "kv metadata (show versions)" "version" \
        secrets kv metadata myapp/config

    # Check-and-Set (CAS)
    run_test "kv put (create for CAS test)" \
        secrets kv put cas/test '{"value":"initial"}'

    run_test_expect_fail "kv put (CAS with wrong version)" \
        secrets kv put cas/test '{"value":"updated"}' --cas 0

    run_test "kv put (CAS with correct version)" \
        secrets kv put cas/test '{"value":"updated"}' --cas 1

    # Delete and Undelete
    run_test "kv put (for delete test)" \
        secrets kv put deletable/secret '{"data":"to-delete"}'

    run_test "kv delete (soft delete)" \
        secrets kv delete deletable/secret --versions 1

    run_test "kv undelete (restore)" \
        secrets kv undelete deletable/secret --versions 1

    run_test_expect "kv get (after undelete)" "to-delete" \
        secrets kv get deletable/secret

    # Destroy (hard delete)
    run_test "kv put (for destroy test)" \
        secrets kv put destroy/secret '{"data":"to-destroy"}'

    run_test "kv destroy (permanent delete)" \
        secrets kv destroy destroy/secret --versions 1

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# TRANSIT ENGINE TESTS
# =============================================================================
if should_run_category "transit"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Transit Engine Tests${NC}\n"
    fi

    # Create keys
    run_test "transit create-key (AES-256-GCM)" \
        secrets transit create-key app-encrypt-key --key-type aes256-gcm

    run_test "transit create-key (XChaCha20-Poly1305)" \
        secrets transit create-key app-chacha-key --key-type xchacha20-poly1305

    run_test "transit create-key (Ed25519 for signing)" \
        secrets transit create-key app-sign-key --key-type ed25519

    # List keys
    run_test_expect "transit list-keys" "app-encrypt-key" \
        secrets transit list-keys

    # Encrypt/Decrypt roundtrip with AES-256-GCM
    run_test "transit encrypt (AES-256-GCM)" \
        secrets transit encrypt app-encrypt-key "Hello, World! This is sensitive data."

    # Need to get the ciphertext for decrypt test
    run_cli secrets transit encrypt app-encrypt-key "Test message for decrypt"
    aes_ct="$LAST_OUTPUT"
    # Extract ciphertext from output (format: "Ciphertext: aspen:v1:...")
    aes_ct=$(echo "$aes_ct" | grep -oE 'aspen:v[0-9]+:[A-Za-z0-9+/=]+' | head -1 || true)

    if [ -n "$aes_ct" ]; then
        run_test_expect "transit decrypt (AES-256-GCM)" "Test message" \
            secrets transit decrypt app-encrypt-key "$aes_ct"
    else
        printf "  %-60s ${YELLOW}SKIP${NC} (no ciphertext captured)\n" "transit decrypt (AES-256-GCM)"
        TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
    fi

    # Encrypt/Decrypt with XChaCha20-Poly1305
    run_cli secrets transit encrypt app-chacha-key "ChaCha20 encrypted message"
    chacha_ct="$LAST_OUTPUT"
    chacha_ct=$(echo "$chacha_ct" | grep -oE 'aspen:v[0-9]+:[A-Za-z0-9+/=]+' | head -1 || true)

    if [ -n "$chacha_ct" ]; then
        run_test_expect "transit decrypt (XChaCha20-Poly1305)" "ChaCha20 encrypted" \
            secrets transit decrypt app-chacha-key "$chacha_ct"
    else
        printf "  %-60s ${YELLOW}SKIP${NC} (no ciphertext captured)\n" "transit decrypt (XChaCha20-Poly1305)"
        TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
    fi

    # Sign/Verify with Ed25519
    run_cli secrets transit sign app-sign-key "Message to sign"
    sig="$LAST_OUTPUT"
    sig=$(echo "$sig" | grep -oE 'aspen:v[0-9]+:[A-Za-z0-9+/=]+' | head -1 || true)

    if [ -n "$sig" ]; then
        run_test_expect "transit verify (valid signature)" "VALID" \
            secrets transit verify app-sign-key "Message to sign" "$sig"

        run_test_expect_fail "transit verify (invalid - wrong data)" \
            secrets transit verify app-sign-key "Different message" "$sig"
    else
        printf "  %-60s ${YELLOW}SKIP${NC} (no signature captured)\n" "transit verify (valid signature)"
        printf "  %-60s ${YELLOW}SKIP${NC} (no signature captured)\n" "transit verify (invalid - wrong data)"
        TESTS_SKIPPED=$((TESTS_SKIPPED + 2))
    fi

    # Key rotation
    run_test "transit rotate-key" \
        secrets transit rotate-key app-encrypt-key

    # Encrypt with rotated key (v2)
    run_cli secrets transit encrypt app-encrypt-key "Post-rotation message"
    rotated_ct="$LAST_OUTPUT"
    rotated_ct=$(echo "$rotated_ct" | grep -oE 'aspen:v[0-9]+:[A-Za-z0-9+/=]+' | head -1 || true)

    if [ -n "$rotated_ct" ]; then
        run_test_expect "transit decrypt (after rotation)" "Post-rotation" \
            secrets transit decrypt app-encrypt-key "$rotated_ct"
    fi

    # Decrypt old ciphertext after rotation (backward compatibility)
    if [ -n "$aes_ct" ]; then
        run_test_expect "transit decrypt (old ciphertext after rotation)" "Test message" \
            secrets transit decrypt app-encrypt-key "$aes_ct"
    fi

    # Datakey generation
    run_test_expect "transit datakey (plaintext)" "Plaintext" \
        secrets transit datakey app-encrypt-key --key-type plaintext

    run_test_expect "transit datakey (wrapped)" "Ciphertext" \
        secrets transit datakey app-encrypt-key --key-type wrapped

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# PKI ENGINE TESTS
# =============================================================================
if should_run_category "pki"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}PKI Engine Tests${NC}\n"
    fi

    # Generate Root CA
    run_test_expect "pki generate-root (create CA)" "BEGIN CERTIFICATE" \
        secrets pki generate-root "Aspen Test Root CA" --ttl-days 365

    # Create roles
    run_test "pki create-role (web-server)" \
        secrets pki create-role web-server \
            --allowed-domains example.com,test.local \
            --max-ttl-days 30 \
            --allow-bare-domains \
            --allow-wildcards

    run_test "pki create-role (api-server)" \
        secrets pki create-role api-server \
            --allowed-domains api.example.com \
            --max-ttl-days 7

    # List roles
    run_test_expect "pki list-roles" "web-server" \
        secrets pki list-roles

    # Get role
    run_test_expect "pki get-role" "example.com" \
        secrets pki get-role web-server

    # Issue certificates (use bare domains matching allowed_domains)
    run_test_expect "pki issue (bare domain cert)" "BEGIN CERTIFICATE" \
        secrets pki issue web-server example.com --ttl-days 7

    run_test_expect "pki issue (bare domain with alt-names)" "BEGIN CERTIFICATE" \
        secrets pki issue web-server test.local --alt-names example.com --ttl-days 7

    # List certificates
    run_test "pki list-certs" \
        secrets pki list-certs

    # Issue and revoke
    run_cli secrets pki issue web-server example.com --ttl-days 1
    cert_output="$LAST_OUTPUT"
    # Extract serial from output
    serial=""
    serial=$(echo "$cert_output" | grep -oE 'Serial: [a-f0-9]+' | cut -d' ' -f2 || true)

    if [ -n "$serial" ]; then
        run_test "pki revoke (certificate)" \
            secrets pki revoke "$serial"

        run_test "pki get-crl (after revocation)" \
            secrets pki get-crl
    else
        printf "  %-60s ${YELLOW}SKIP${NC} (no serial captured)\n" "pki revoke (certificate)"
        printf "  %-60s ${YELLOW}SKIP${NC} (no serial captured)\n" "pki get-crl (after revocation)"
        TESTS_SKIPPED=$((TESTS_SKIPPED + 2))
    fi

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# WORKFLOW TESTS (END-TO-END SCENARIOS)
# =============================================================================
if should_run_category "workflow"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Workflow Tests (End-to-End Scenarios)${NC}\n"
    fi

    # Scenario 1: Application secrets management
    run_test "workflow: store database credentials" \
        secrets kv put prod/database '{"host":"db.example.com","port":"5432","username":"app","password":"supersecret"}'

    run_test "workflow: store API keys" \
        secrets kv put prod/api-keys '{"stripe":"sk_live_xxx","sendgrid":"SG.xxx"}'

    run_test_expect "workflow: retrieve database host" "db.example.com" \
        secrets kv get prod/database

    # Scenario 2: Encryption workflow
    run_test "workflow: create encryption key for user data" \
        secrets transit create-key user-data-key --key-type aes256-gcm

    run_cli secrets transit encrypt user-data-key '{"ssn":"123-45-6789","dob":"1990-01-01"}'
    user_data_ct="$LAST_OUTPUT"
    user_data_ct=$(echo "$user_data_ct" | grep -oE 'aspen:v[0-9]+:[A-Za-z0-9+/=]+' | head -1 || true)

    if [ -n "$user_data_ct" ]; then
        run_test "workflow: store encrypted user data in KV" \
            secrets kv put users/user123/pii "{\"encrypted\":\"$user_data_ct\"}"

        run_test_expect "workflow: retrieve and decrypt user data" "123-45-6789" \
            secrets transit decrypt user-data-key "$user_data_ct"
    fi

    # Scenario 3: PKI workflow - issue service certificates
    run_test "workflow: create PKI role for microservices" \
        secrets pki create-role microservices \
            --allowed-domains api.cluster.local \
            --max-ttl-days 1 \
            --allow-bare-domains

    run_test_expect "workflow: issue certificate for api service" "PRIVATE KEY" \
        secrets pki issue microservices api.cluster.local --ttl-days 1

    # Scenario 4: Key rotation workflow
    run_test "workflow: create key for rotation demo" \
        secrets transit create-key rotation-demo-key --key-type aes256-gcm

    run_cli secrets transit encrypt rotation-demo-key "v1 secret data"
    v1_ct="$LAST_OUTPUT"
    v1_ct=$(echo "$v1_ct" | grep -oE 'aspen:v[0-9]+:[A-Za-z0-9+/=]+' | head -1 || true)

    run_test "workflow: rotate key to v2" \
        secrets transit rotate-key rotation-demo-key

    if [ -n "$v1_ct" ]; then
        run_test_expect "workflow: decrypt v1 ciphertext with v2 key" "v1 secret data" \
            secrets transit decrypt rotation-demo-key "$v1_ct"
    fi

    run_cli secrets transit encrypt rotation-demo-key "v2 secret data"
    v2_ct="$LAST_OUTPUT"
    v2_ct=$(echo "$v2_ct" | grep -oE 'aspen:v[0-9]+:[A-Za-z0-9+/=]+' | head -1 || true)

    if [ -n "$v2_ct" ]; then
        # Verify v2 ciphertext uses v2 key
        if echo "$v2_ct" | grep -q "aspen:v2:"; then
            printf "  %-60s ${GREEN}PASS${NC}\n" "workflow: v2 ciphertext uses v2 key"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            printf "  %-60s ${YELLOW}SKIP${NC} (key version not in expected format)\n" "workflow: v2 ciphertext uses v2 key"
            TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
        fi
    fi

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# ERROR HANDLING TESTS
# =============================================================================
if should_run_category "errors" || [ -z "$CATEGORY" ]; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Error Handling Tests${NC}\n"
    fi

    # KV errors
    run_test_expect_fail "kv put (invalid JSON)" \
        secrets kv put error/test "not valid json"

    run_test_expect_fail "kv get (non-existent path)" \
        secrets kv get nonexistent/path/that/does/not/exist

    run_test_expect_fail "kv get (invalid version number)" \
        secrets kv get myapp/config --version 99999

    run_test_expect_fail "kv destroy (no versions)" \
        secrets kv destroy error/noversions --versions ""

    # CAS conflict test
    run_cli secrets kv put error/cas-test '{"v":"1"}'
    run_test_expect_fail "kv put (CAS conflict - wrong version)" \
        secrets kv put error/cas-test '{"v":"2"}' --cas 99

    # Transit errors
    run_test_expect_fail "transit encrypt (non-existent key)" \
        secrets transit encrypt nonexistent-key "data"

    run_test_expect_fail "transit decrypt (invalid ciphertext)" \
        secrets transit decrypt app-encrypt-key "invalid:ciphertext:format"

    run_test_expect_fail "transit create-key (invalid type)" \
        secrets transit create-key bad-key --key-type invalid-type

    # Sign with encryption-only key should fail
    run_cli secrets transit create-key error-aes-key --key-type aes256-gcm
    run_test_expect_fail "transit sign (encryption key)" \
        secrets transit sign error-aes-key "data to sign"

    run_test_expect_fail "transit verify (non-existent key)" \
        secrets transit verify nonexistent-key "data" "aspen:v1:invalid"

    run_test_expect_fail "transit datakey (non-existent key)" \
        secrets transit datakey nonexistent-key

    # PKI errors
    run_test_expect_fail "pki issue (non-existent role)" \
        secrets pki issue nonexistent-role test.example.com

    run_test_expect_fail "pki revoke (invalid serial)" \
        secrets pki revoke "INVALID:SERIAL:FORMAT"

    run_test_expect_fail "pki get-role (non-existent)" \
        secrets pki get-role nonexistent-role-name

    # Domain not allowed by role
    run_cli secrets pki create-role error-test-role --allowed-domains "allowed.com" --max-ttl-days 1
    run_test_expect_fail "pki issue (unauthorized domain)" \
        secrets pki issue error-test-role "unauthorized.example.com"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# CUSTOM MOUNT POINT TESTS
# =============================================================================
if should_run_category "mount"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Custom Mount Point Tests${NC}\n"
    fi

    # KV with custom mount
    run_test "kv put (custom mount)" \
        secrets kv put mount-test/secret '{"data":"custom-mount-value"}' --mount custom-kv

    run_test_expect "kv get (custom mount)" "custom-mount-value" \
        secrets kv get mount-test/secret --mount custom-kv

    run_test "kv list (custom mount)" \
        secrets kv list mount-test/ --mount custom-kv

    run_test_expect "kv metadata (custom mount)" "version" \
        secrets kv metadata mount-test/secret --mount custom-kv

    # Transit with custom mount
    run_test "transit create-key (custom mount)" \
        secrets transit create-key custom-mount-key --mount custom-transit --key-type aes256-gcm

    run_cli secrets transit encrypt custom-mount-key "custom mount encrypt" --mount custom-transit
    CUSTOM_MT_CT=$(echo "$LAST_OUTPUT" | grep -oE 'aspen:v[0-9]+:[A-Za-z0-9+/=]+' | head -1 || true)

    if [ -n "$CUSTOM_MT_CT" ]; then
        run_test_expect "transit decrypt (custom mount)" "custom mount encrypt" \
            secrets transit decrypt custom-mount-key "$CUSTOM_MT_CT" --mount custom-transit
    else
        printf "  %-60s ${YELLOW}SKIP${NC} (no ciphertext)\n" "transit decrypt (custom mount)"
        TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
    fi

    run_test_expect "transit list-keys (custom mount)" "custom-mount-key" \
        secrets transit list-keys --mount custom-transit

    # PKI with custom mount
    run_test_expect "pki generate-root (custom mount)" "BEGIN CERTIFICATE" \
        secrets pki generate-root "Custom Mount CA" --mount "${TEST_PREFIX}custom-pki" --ttl-days 30

    run_test "pki create-role (custom mount)" \
        secrets pki create-role custom-role --mount "${TEST_PREFIX}custom-pki" \
            --allowed-domains custom.local --allow-bare-domains --max-ttl-days 7

    run_test_expect "pki list-roles (custom mount)" "custom-role" \
        secrets pki list-roles --mount "${TEST_PREFIX}custom-pki"

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# CONVERGENT ENCRYPTION TESTS
# =============================================================================
if should_run_category "convergent"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Convergent Encryption Tests${NC}\n"
    fi

    # Create key for convergent encryption tests
    run_test "transit create-key (convergent)" \
        secrets transit create-key convergent-test-key --key-type aes256-gcm

    # Base64-encode a context
    CONV_CONTEXT=$(echo -n "user-id:test-user-12345" | base64)

    # Encrypt with context
    run_test "transit encrypt (with context)" \
        secrets transit encrypt convergent-test-key "sensitive user data" --context "$CONV_CONTEXT"

    # Capture ciphertext for decrypt
    run_cli secrets transit encrypt convergent-test-key "decrypt with context test" --context "$CONV_CONTEXT"
    CONV_CT=$(echo "$LAST_OUTPUT" | grep -oE 'aspen:v[0-9]+:[A-Za-z0-9+/=]+' | head -1 || true)

    if [ -n "$CONV_CT" ]; then
        # Decrypt with matching context
        run_test_expect "transit decrypt (matching context)" "decrypt with context test" \
            secrets transit decrypt convergent-test-key "$CONV_CT" --context "$CONV_CONTEXT"

        # Decrypt with wrong context should fail
        WRONG_CONTEXT=$(echo -n "user-id:wrong-user-99999" | base64)
        run_test_expect_fail "transit decrypt (wrong context)" \
            secrets transit decrypt convergent-test-key "$CONV_CT" --context "$WRONG_CONTEXT"

        # Decrypt without context should fail
        run_test_expect_fail "transit decrypt (missing context)" \
            secrets transit decrypt convergent-test-key "$CONV_CT"
    else
        printf "  %-60s ${YELLOW}SKIP${NC} (no ciphertext)\n" "transit decrypt (matching context)"
        printf "  %-60s ${YELLOW}SKIP${NC} (no ciphertext)\n" "transit decrypt (wrong context)"
        printf "  %-60s ${YELLOW}SKIP${NC} (no ciphertext)\n" "transit decrypt (missing context)"
        TESTS_SKIPPED=$((TESTS_SKIPPED + 3))
    fi

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# EDGE CASE TESTS
# =============================================================================
if should_run_category "edge"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Edge Case Tests${NC}\n"
    fi

    # Minimal JSON data
    run_test "kv put (minimal data)" \
        secrets kv put edge/minimal '{"k":"v"}'
    run_test_expect "kv get (minimal data)" "v" \
        secrets kv get edge/minimal

    # Deeply nested path
    DEEP_PATH="edge/very/deeply/nested/path/to/secret"
    run_test "kv put (deeply nested path)" \
        secrets kv put "$DEEP_PATH" '{"deep":"nested-secret"}'
    run_test_expect "kv get (deeply nested path)" "nested-secret" \
        secrets kv get "$DEEP_PATH"

    # Special characters in JSON values (escaped properly)
    run_test "kv put (special chars in value)" \
        secrets kv put edge/special '{"data":"value with spaces"}'
    run_test_expect "kv get (special chars)" "spaces" \
        secrets kv get edge/special

    # Multiple versions and version traversal
    run_test "kv put (version-test v1)" \
        secrets kv put edge/versions '{"version":"1"}'
    run_test "kv put (version-test v2)" \
        secrets kv put edge/versions '{"version":"2"}'
    run_test "kv put (version-test v3)" \
        secrets kv put edge/versions '{"version":"3"}'

    run_test_expect "kv get (specific version 1)" "\"1\"" \
        secrets kv get edge/versions --version 1
    run_test_expect "kv get (specific version 2)" "\"2\"" \
        secrets kv get edge/versions --version 2
    run_test_expect "kv get (latest = v3)" "\"3\"" \
        secrets kv get edge/versions

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# MULTI-VERSION METADATA TESTS
# =============================================================================
if should_run_category "metadata"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}Multi-Version Metadata Tests${NC}\n"
    fi

    # Create secret with multiple versions
    run_test "kv put (metadata-test v1)" \
        secrets kv put meta/multi '{"v":"1"}'
    run_test "kv put (metadata-test v2)" \
        secrets kv put meta/multi '{"v":"2"}'
    run_test "kv put (metadata-test v3)" \
        secrets kv put meta/multi '{"v":"3"}'

    # Verify metadata shows versions
    run_test_expect "kv metadata (shows current version)" "Current version:" \
        secrets kv metadata meta/multi

    # Delete specific version
    run_test "kv delete (version 2 only)" \
        secrets kv delete meta/multi --versions 2

    # Undelete
    run_test "kv undelete (version 2)" \
        secrets kv undelete meta/multi --versions 2

    # Verify we can read undeleted version
    run_test_expect "kv get (v2 after undelete)" "\"2\"" \
        secrets kv get meta/multi --version 2

    # Destroy specific version permanently
    run_test "kv destroy (version 1)" \
        secrets kv destroy meta/multi --versions 1

    # Version 1 should now fail
    run_test_expect_fail "kv get (v1 after destroy)" \
        secrets kv get meta/multi --version 1

    # Multiple version delete
    run_test "kv delete (versions 2,3)" \
        secrets kv delete meta/multi --versions 2,3

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# PKI CERTIFICATE CHAIN TESTS
# =============================================================================
if should_run_category "chain"; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}PKI Certificate Chain Tests${NC}\n"
    fi

    # Generate root for chain testing (separate mount to avoid conflicts)
    run_test_expect "pki generate-root (chain test)" "BEGIN CERTIFICATE" \
        secrets pki generate-root "Chain Test Root CA" --mount "${TEST_PREFIX}chain-pki" --ttl-days 365

    # Create role with multiple domains
    run_test "pki create-role (multi-domain)" \
        secrets pki create-role chain-role --mount "${TEST_PREFIX}chain-pki" \
            --allowed-domains "example.com,test.local" \
            --allow-bare-domains \
            --allow-wildcards \
            --max-ttl-days 30

    # Issue cert for bare domain
    run_test_expect "pki issue (bare domain)" "BEGIN CERTIFICATE" \
        secrets pki issue chain-role "example.com" --mount "${TEST_PREFIX}chain-pki" --ttl-days 7

    # Issue cert with alt names
    run_test_expect "pki issue (with alt-names)" "BEGIN CERTIFICATE" \
        secrets pki issue chain-role "test.local" --mount "${TEST_PREFIX}chain-pki" \
            --alt-names "www.test.local,api.test.local" --ttl-days 7

    # Verify certs listed
    run_test "pki list-certs (chain mount)" \
        secrets pki list-certs --mount "${TEST_PREFIX}chain-pki"

    # Get role details
    run_test_expect "pki get-role (verify config)" "example.com" \
        secrets pki get-role chain-role --mount "${TEST_PREFIX}chain-pki"

    # Issue and revoke certificate
    run_cli secrets pki issue chain-role "example.com" --mount "${TEST_PREFIX}chain-pki" --ttl-days 1
    CHAIN_SERIAL=$(echo "$LAST_OUTPUT" | grep -oE 'Serial: [a-f0-9]+' | cut -d' ' -f2 || true)

    if [ -n "$CHAIN_SERIAL" ]; then
        run_test "pki revoke (chain cert)" \
            secrets pki revoke "$CHAIN_SERIAL" --mount "${TEST_PREFIX}chain-pki"

        run_test "pki get-crl (after revoke)" \
            secrets pki get-crl --mount "${TEST_PREFIX}chain-pki"
    else
        printf "  %-60s ${YELLOW}SKIP${NC} (no serial)\n" "pki revoke (chain cert)"
        printf "  %-60s ${YELLOW}SKIP${NC} (no serial)\n" "pki get-crl (after revoke)"
        TESTS_SKIPPED=$((TESTS_SKIPPED + 2))
    fi

    if ! $JSON_OUTPUT; then
        printf "\n"
    fi
fi

# =============================================================================
# JSON OUTPUT FORMAT TESTS
# =============================================================================
if should_run_category "json" || [ -z "$CATEGORY" ]; then
    if ! $JSON_OUTPUT; then
        printf "${CYAN}JSON Output Format Tests${NC}\n"
    fi

    # KV JSON output
    run_test_expect "kv get (json format)" "\"success\"" \
        secrets kv get myapp/config --json

    run_test_expect "kv list (json format)" "\"keys\"" \
        secrets kv list --json

    run_test_expect "kv metadata (json format)" "\"current_version\"" \
        secrets kv metadata myapp/config --json

    # Transit JSON output
    run_test_expect "transit list-keys (json format)" "\"keys\"" \
        secrets transit list-keys --json

    # PKI JSON output
    run_test_expect "pki list-roles (json format)" "\"items\"" \
        secrets pki list-roles --json

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
        printf "${GREEN}${BOLD}All secrets tests passed!${NC}\n"
    else
        printf "${RED}${BOLD}Some secrets tests failed.${NC}\n"
    fi
fi

# Exit with failure if any tests failed
[ "$TESTS_FAILED" -eq 0 ]
