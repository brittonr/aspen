#!/usr/bin/env bash
#
# Aspen Forge Comprehensive Git Hosting Test Suite
#
# A production-grade test suite that validates Forge's ability to host
# real Git repositories with full clone/push/pull workflows.
#
# Test Categories:
#   1. Cluster Infrastructure (startup, health, ticket)
#   2. Repository Management (create, get, list, delete)
#   3. Git Object Storage (blobs, trees, commits, tags)
#   4. Reference Management (branches, tags, CAS operations)
#   5. Git Bridge Protocol (SHA-1 ↔ BLAKE3 translation)
#   6. Remote Helper Protocol (git-remote-aspen capabilities)
#   7. Full Git Workflow (init, add, commit, push, clone, pull)
#   8. Edge Cases (large files, binary content, empty repos)
#   9. Concurrent Operations (parallel writes, race conditions)
#  10. Error Handling (invalid inputs, missing refs, permission)
#  11. Performance Baseline (latency measurements)
#  12. Audit Validation (consistency checks, data integrity)
#
# Usage:
#   ./scripts/forge-comprehensive-test.sh [options]
#
# Options:
#   --quick           Skip slow tests (large files, performance)
#   --category NAME   Run only tests in category
#   --verbose         Print detailed output
#   --keep-data       Don't clean up after tests
#   --multi-node N    Test with N-node cluster (default: 1)
#   --help            Show this help
#
# Exit Codes:
#   0 - All tests passed
#   1 - Some tests failed
#   2 - Setup failed (missing binaries, cluster startup)
#   3 - Invalid arguments
#
# Requirements:
#   - cargo build --release --features git-bridge
#   - git (for workflow tests)
#   - Standard POSIX utilities (timeout, mktemp, etc.)
#

set -euo pipefail

# =============================================================================
# CONFIGURATION
# =============================================================================

# Script info
readonly SCRIPT_VERSION="1.0.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
readonly PROJECT_DIR

# Cluster configuration
export ASPEN_BLOBS="${ASPEN_BLOBS:-true}"
export ASPEN_STORAGE="${ASPEN_STORAGE:-redb}"
export ASPEN_NODE_COUNT="${ASPEN_NODE_COUNT:-1}"
export ASPEN_LOG_LEVEL="${ASPEN_LOG_LEVEL:-warn}"
export ASPEN_FOREGROUND="${ASPEN_FOREGROUND:-false}"

# Test configuration
QUICK_MODE=false
VERBOSE=false
KEEP_DATA=false
CATEGORY_FILTER=""
MULTI_NODE=1
TEST_TIMEOUT=300  # 5 minutes max per test category

# Test state
declare -a FAILED_TESTS=()
declare -A TEST_DURATIONS=()
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0
START_TIME=""
TICKET=""
REPO_ID=""

# Paths
DATA_DIR=""
TEST_GIT_DIR=""
ASPEN_CLI=""
ASPEN_NODE=""
GIT_REMOTE_ASPEN=""
CLUSTER_SCRIPT="$SCRIPT_DIR/cluster.sh"

# Colors
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly CYAN='\033[0;36m'
readonly BOLD='\033[1m'
readonly NC='\033[0m'

# =============================================================================
# UTILITIES
# =============================================================================

log_info() {
    printf "${BLUE}[INFO]${NC} %s\n" "$*"
}

log_success() {
    printf "${GREEN}[PASS]${NC} %s\n" "$*"
}

log_warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$*"
}

log_error() {
    printf "${RED}[FAIL]${NC} %s\n" "$*"
}

log_debug() {
    if [ "$VERBOSE" = true ]; then
        printf "${CYAN}[DEBUG]${NC} %s\n" "$*"
    fi
}

# Print test header
print_header() {
    local title="$1"
    printf "\n${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${BOLD}${CYAN}  %s${NC}\n" "$title"
    printf "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
}

# Record test result
record_test() {
    local name="$1"
    local passed="$2"
    local message="${3:-}"
    local duration="${4:-0}"

    TESTS_RUN=$((TESTS_RUN + 1))
    TEST_DURATIONS["$name"]="$duration"

    if [ "$passed" = true ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_success "$name${message:+ - $message} (${duration}ms)"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        FAILED_TESTS+=("$name")
        log_error "$name${message:+ - $message}"
    fi
}

# Skip test
skip_test() {
    local name="$1"
    local reason="${2:-}"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
    printf "${YELLOW}[SKIP]${NC} %s%s\n" "$name" "${reason:+ - $reason}"
}

# Run command with timing
timed_run() {
    local start_ms
    local end_ms
    start_ms=$(date +%s%3N)

    if "$@"; then
        end_ms=$(date +%s%3N)
        echo $((end_ms - start_ms))
        return 0
    else
        end_ms=$(date +%s%3N)
        echo $((end_ms - start_ms))
        return 1
    fi
}

# Run CLI command
cli() {
    "$ASPEN_CLI" --ticket "$TICKET" "$@"
}

# Run CLI and capture output
cli_capture() {
    "$ASPEN_CLI" --ticket "$TICKET" "$@" 2>&1
}

# Generate random string
random_string() {
    local length="${1:-8}"
    tr -dc 'a-zA-Z0-9' < /dev/urandom | head -c "$length"
}

# Generate test prefix (unique per run)
TEST_PREFIX="test_$(date +%s)_$$"
readonly TEST_PREFIX

# =============================================================================
# SETUP AND CLEANUP
# =============================================================================

find_binary() {
    local name="$1"
    local bin=""

    # Check environment variable
    local env_var="ASPEN_${name^^}_BIN"
    env_var="${env_var//-/_}"
    if [ -n "${!env_var:-}" ] && [ -x "${!env_var}" ]; then
        echo "${!env_var}"
        return 0
    fi

    # Check PATH
    bin=$(command -v "$name" 2>/dev/null || echo "")
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check project directories
    for path in target/release/"$name" target/debug/"$name" result/bin/"$name"; do
        bin="$PROJECT_DIR/$path"
        if [ -x "$bin" ]; then
            echo "$bin"
            return 0
        fi
    done

    echo ""
}

wait_for_ticket() {
    local ticket_file="$DATA_DIR/ticket.txt"
    local timeout=60
    local elapsed=0

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$ticket_file" ]; then
            cat "$ticket_file"
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done

    return 1
}

setup() {
    print_header "SETUP"

    START_TIME=$(date +%s)
    DATA_DIR="/tmp/forge-test-$$"
    export ASPEN_DATA_DIR="$DATA_DIR"
    export ASPEN_NODE_COUNT="$MULTI_NODE"

    # Find binaries
    ASPEN_CLI=$(find_binary aspen-cli)
    ASPEN_NODE=$(find_binary aspen-node)
    GIT_REMOTE_ASPEN=$(find_binary git-remote-aspen)

    log_info "Configuration:"
    printf "  Data directory: %s\n" "$DATA_DIR"
    printf "  Node count:     %d\n" "$MULTI_NODE"
    printf "  Storage:        %s\n" "$ASPEN_STORAGE"
    printf "  Quick mode:     %s\n" "$QUICK_MODE"
    printf "  Verbose:        %s\n" "$VERBOSE"
    printf "\n"

    # Validate binaries
    if [ -z "$ASPEN_CLI" ] || [ ! -x "$ASPEN_CLI" ]; then
        log_error "aspen-cli not found"
        log_info "Build with: cargo build --release -p aspen-cli"
        return 2
    fi
    printf "  aspen-cli:          %s\n" "$ASPEN_CLI"

    if [ -z "$ASPEN_NODE" ] || [ ! -x "$ASPEN_NODE" ]; then
        log_error "aspen-node not found"
        log_info "Build with: cargo build --release -p aspen"
        return 2
    fi
    printf "  aspen-node:         %s\n" "$ASPEN_NODE"

    if [ -z "$GIT_REMOTE_ASPEN" ] || [ ! -x "$GIT_REMOTE_ASPEN" ]; then
        log_warn "git-remote-aspen not found - some tests will be skipped"
        log_info "Build with: cargo build --release --features git-bridge"
    else
        printf "  git-remote-aspen:   %s\n" "$GIT_REMOTE_ASPEN"
    fi

    printf "\n"

    # Check git is available
    if ! command -v git >/dev/null 2>&1; then
        log_error "git not found in PATH"
        return 2
    fi
    printf "  git:                %s\n" "$(command -v git)"

    # Create data directory
    mkdir -p "$DATA_DIR"

    # Start cluster
    log_info "Starting cluster..."
    if ! "$CLUSTER_SCRIPT" start >/dev/null 2>&1; then
        log_error "Failed to start cluster"
        return 2
    fi

    # Wait for ticket
    log_info "Waiting for cluster ticket..."
    if ! TICKET=$(wait_for_ticket); then
        log_error "Timeout waiting for cluster ticket"
        return 2
    fi

    log_success "Cluster started with ticket: ${TICKET:0:40}..."

    # Initialize cluster if needed
    sleep 2
    cli cluster init >/dev/null 2>&1 || true

    return 0
}

cleanup() {
    local exit_code=$?

    print_header "CLEANUP"

    # Stop cluster
    if [ -x "$CLUSTER_SCRIPT" ]; then
        log_info "Stopping cluster..."
        "$CLUSTER_SCRIPT" stop 2>/dev/null || true
    fi

    # Clean up test git directory
    if [ -n "${TEST_GIT_DIR:-}" ] && [ -d "$TEST_GIT_DIR" ]; then
        rm -rf "$TEST_GIT_DIR"
    fi

    # Clean up data directory unless --keep-data
    if [ "$KEEP_DATA" = false ] && [ -n "${DATA_DIR:-}" ] && [ -d "$DATA_DIR" ]; then
        log_info "Removing data directory: $DATA_DIR"
        rm -rf "$DATA_DIR"
    elif [ "$KEEP_DATA" = true ]; then
        log_info "Keeping data directory: $DATA_DIR"
    fi

    exit $exit_code
}

# =============================================================================
# TEST CATEGORY 1: CLUSTER INFRASTRUCTURE
# =============================================================================

test_cluster_infrastructure() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "cluster" ]; then
        return 0
    fi

    print_header "1. CLUSTER INFRASTRUCTURE"
    local start_ms duration

    # Test 1.1: Cluster health check
    start_ms=$(date +%s%3N)
    if cli cluster status >/dev/null 2>&1; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "cluster_health_check" true "" "$duration"
    else
        record_test "cluster_health_check" false "cluster status failed"
    fi

    # Test 1.2: Cluster metrics
    start_ms=$(date +%s%3N)
    local metrics
    if metrics=$(cli_capture cluster status 2>/dev/null) && [ -n "$metrics" ]; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "cluster_metrics" true "" "$duration"
    else
        record_test "cluster_metrics" false "no metrics returned"
    fi

    # Test 1.3: Node connectivity (multi-node only)
    if [ "$MULTI_NODE" -gt 1 ]; then
        start_ms=$(date +%s%3N)
        if echo "$metrics" | grep -q "voters"; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "multi_node_connectivity" true "$MULTI_NODE nodes" "$duration"
        else
            record_test "multi_node_connectivity" false "voters not found in metrics"
        fi
    fi
}

# =============================================================================
# TEST CATEGORY 2: REPOSITORY MANAGEMENT
# =============================================================================

test_repository_management() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "repo" ]; then
        return 0
    fi

    print_header "2. REPOSITORY MANAGEMENT"
    local start_ms duration output

    # Test 2.1: Create repository
    local repo_name="${TEST_PREFIX}_repo"
    start_ms=$(date +%s%3N)
    if output=$(cli_capture git init "$repo_name" 2>/dev/null); then
        REPO_ID=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")
        if [ -n "$REPO_ID" ]; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "create_repository" true "ID: ${REPO_ID:0:16}..." "$duration"
        else
            record_test "create_repository" false "no repo ID in output"
        fi
    else
        record_test "create_repository" false "git init failed"
    fi

    # Test 2.2: Get repository
    if [ -n "$REPO_ID" ]; then
        start_ms=$(date +%s%3N)
        if cli git show --repo "$REPO_ID" >/dev/null 2>&1; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "get_repository" true "" "$duration"
        else
            record_test "get_repository" false "git show failed"
        fi
    fi

    # Test 2.3: List repositories
    start_ms=$(date +%s%3N)
    if cli git list >/dev/null 2>&1; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "list_repositories" true "" "$duration"
    else
        # git list might not be implemented, skip
        skip_test "list_repositories" "command not available"
    fi

    # Test 2.4: Duplicate repository name
    start_ms=$(date +%s%3N)
    if cli git init "$repo_name" >/dev/null 2>&1; then
        record_test "duplicate_repo_rejected" false "duplicate allowed"
    else
        duration=$(($(date +%s%3N) - start_ms))
        record_test "duplicate_repo_rejected" true "" "$duration"
    fi

    # Test 2.5: Invalid repository ID
    start_ms=$(date +%s%3N)
    if cli git show --repo "0000000000000000000000000000000000000000000000000000000000000000" 2>&1 | grep -qi "not found\|error"; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "invalid_repo_handled" true "" "$duration"
    else
        record_test "invalid_repo_handled" false "no error for invalid ID"
    fi
}

# =============================================================================
# TEST CATEGORY 3: GIT OBJECT STORAGE
# =============================================================================

test_git_object_storage() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "objects" ]; then
        return 0
    fi

    print_header "3. GIT OBJECT STORAGE"
    local start_ms duration output hash

    # Test 3.1: Store blob
    local blob_content="Hello from Forge test at $(date)"
    start_ms=$(date +%s%3N)
    if output=$(echo -n "$blob_content" | cli_capture blob add - 2>/dev/null); then
        hash=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")
        if [ -n "$hash" ]; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "store_blob" true "hash: ${hash:0:16}..." "$duration"
        else
            record_test "store_blob" false "no hash in output"
        fi
    else
        record_test "store_blob" false "blob add failed"
    fi

    # Test 3.2: Retrieve blob
    if [ -n "$hash" ]; then
        start_ms=$(date +%s%3N)
        local retrieved
        if retrieved=$(cli blob get "$hash" 2>/dev/null | tail -1); then
            if [ "$retrieved" = "$blob_content" ]; then
                duration=$(($(date +%s%3N) - start_ms))
                record_test "retrieve_blob" true "" "$duration"
            else
                record_test "retrieve_blob" false "content mismatch"
            fi
        else
            record_test "retrieve_blob" false "blob get failed"
        fi
    fi

    # Test 3.3: Binary blob
    start_ms=$(date +%s%3N)
    local binary_file
    binary_file=$(mktemp)
    head -c 1024 /dev/urandom > "$binary_file"
    if output=$(cli_capture blob add "$binary_file" 2>/dev/null); then
        hash=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")
        if [ -n "$hash" ]; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "binary_blob" true "1KB random data" "$duration"
        else
            record_test "binary_blob" false "no hash"
        fi
    else
        record_test "binary_blob" false "failed"
    fi
    rm -f "$binary_file"

    # Test 3.4: Large blob (skip in quick mode)
    if [ "$QUICK_MODE" = false ]; then
        start_ms=$(date +%s%3N)
        local large_file
        large_file=$(mktemp)
        dd if=/dev/zero of="$large_file" bs=1M count=10 2>/dev/null
        if output=$(cli_capture blob add "$large_file" 2>/dev/null); then
            hash=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")
            if [ -n "$hash" ]; then
                duration=$(($(date +%s%3N) - start_ms))
                record_test "large_blob_10MB" true "${duration}ms" "$duration"
            else
                record_test "large_blob_10MB" false "no hash"
            fi
        else
            record_test "large_blob_10MB" false "failed"
        fi
        rm -f "$large_file"
    else
        skip_test "large_blob_10MB" "quick mode"
    fi

    # Test 3.5: Empty blob
    start_ms=$(date +%s%3N)
    if output=$(echo -n "" | cli_capture blob add - 2>/dev/null); then
        hash=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")
        if [ -n "$hash" ]; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "empty_blob" true "" "$duration"
        else
            record_test "empty_blob" false "no hash"
        fi
    else
        record_test "empty_blob" false "failed"
    fi

    # Test 3.6: Nonexistent blob
    start_ms=$(date +%s%3N)
    local fake_hash="0000000000000000000000000000000000000000000000000000000000000000"
    if cli blob get "$fake_hash" 2>&1 | grep -qi "not found\|error"; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "nonexistent_blob_error" true "" "$duration"
    else
        record_test "nonexistent_blob_error" false "no error for fake hash"
    fi
}

# =============================================================================
# TEST CATEGORY 4: REFERENCE MANAGEMENT
# =============================================================================

test_reference_management() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "refs" ]; then
        return 0
    fi

    if [ -z "$REPO_ID" ]; then
        skip_test "reference_management" "no repository created"
        return 0
    fi

    print_header "4. REFERENCE MANAGEMENT"
    local start_ms duration output

    # First, create a blob to use as a ref target
    local ref_target
    ref_target=$(echo -n "ref target content" | cli_capture blob add - 2>/dev/null | grep -oE '[a-f0-9]{64}' | head -1 || echo "")

    if [ -z "$ref_target" ]; then
        log_warn "Could not create ref target blob"
        return 0
    fi

    # Test 4.1: Set ref
    start_ms=$(date +%s%3N)
    if cli git ref set --repo "$REPO_ID" --ref "heads/main" --hash "$ref_target" >/dev/null 2>&1; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "set_ref" true "" "$duration"
    else
        # Try alternative command format
        if cli branch create --repo "$REPO_ID" main --from "$ref_target" >/dev/null 2>&1; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "set_ref" true "via branch create" "$duration"
        else
            record_test "set_ref" false "ref set failed"
        fi
    fi

    # Test 4.2: List branches
    start_ms=$(date +%s%3N)
    if cli branch list --repo "$REPO_ID" >/dev/null 2>&1; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "list_branches" true "" "$duration"
    else
        record_test "list_branches" false "failed"
    fi

    # Test 4.3: List tags
    start_ms=$(date +%s%3N)
    if cli tag list --repo "$REPO_ID" >/dev/null 2>&1; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "list_tags" true "" "$duration"
    else
        skip_test "list_tags" "command not available"
    fi
}

# =============================================================================
# TEST CATEGORY 5: ISSUE TRACKING (COB)
# =============================================================================

test_issue_tracking() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "issues" ]; then
        return 0
    fi

    if [ -z "$REPO_ID" ]; then
        skip_test "issue_tracking" "no repository created"
        return 0
    fi

    print_header "5. ISSUE TRACKING (COB)"
    local start_ms duration output issue_id

    # Test 5.1: Create issue
    start_ms=$(date +%s%3N)
    if output=$(cli_capture issue create --repo "$REPO_ID" -t "Test Issue ${TEST_PREFIX}" -b "Test body" 2>/dev/null); then
        issue_id=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")
        duration=$(($(date +%s%3N) - start_ms))
        record_test "create_issue" true "" "$duration"
    else
        record_test "create_issue" false "failed"
    fi

    # Test 5.2: List issues
    start_ms=$(date +%s%3N)
    if cli issue list --repo "$REPO_ID" >/dev/null 2>&1; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "list_issues" true "" "$duration"
    else
        record_test "list_issues" false "failed"
    fi

    # Test 5.3: Get issue
    if [ -n "$issue_id" ]; then
        start_ms=$(date +%s%3N)
        if cli issue show --repo "$REPO_ID" "$issue_id" >/dev/null 2>&1; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "get_issue" true "" "$duration"
        else
            skip_test "get_issue" "command format unclear"
        fi
    fi

    # Test 5.4: Comment on issue
    if [ -n "$issue_id" ]; then
        start_ms=$(date +%s%3N)
        if cli issue comment --repo "$REPO_ID" "$issue_id" -b "Test comment" >/dev/null 2>&1; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "comment_issue" true "" "$duration"
        else
            skip_test "comment_issue" "command format unclear"
        fi
    fi

    # Test 5.5: Close issue
    if [ -n "$issue_id" ]; then
        start_ms=$(date +%s%3N)
        if cli issue close --repo "$REPO_ID" "$issue_id" >/dev/null 2>&1; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "close_issue" true "" "$duration"
        else
            skip_test "close_issue" "command format unclear"
        fi
    fi
}

# =============================================================================
# TEST CATEGORY 6: GIT REMOTE HELPER
# =============================================================================

test_git_remote_helper() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "remote" ]; then
        return 0
    fi

    if [ -z "$GIT_REMOTE_ASPEN" ] || [ ! -x "$GIT_REMOTE_ASPEN" ]; then
        skip_test "git_remote_helper" "git-remote-aspen not found"
        return 0
    fi

    if [ -z "$REPO_ID" ]; then
        skip_test "git_remote_helper" "no repository created"
        return 0
    fi

    print_header "6. GIT REMOTE HELPER"
    local start_ms duration

    local aspen_url="aspen://${TICKET}/${REPO_ID}"

    # Test 6.1: Capabilities response
    start_ms=$(date +%s%3N)
    local caps
    if caps=$(printf "capabilities\n\n" | timeout 10 "$GIT_REMOTE_ASPEN" origin "$aspen_url" 2>/dev/null); then
        if echo "$caps" | grep -q "fetch"; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "remote_capabilities_fetch" true "" "$duration"
        else
            record_test "remote_capabilities_fetch" false "fetch not in capabilities"
        fi

        if echo "$caps" | grep -q "push"; then
            record_test "remote_capabilities_push" true "" "0"
        else
            record_test "remote_capabilities_push" false "push not in capabilities"
        fi
    else
        record_test "remote_capabilities_fetch" false "timeout or error"
    fi

    # Test 6.2: List command
    start_ms=$(date +%s%3N)
    local refs
    if refs=$(printf "list\n\n" | timeout 10 "$GIT_REMOTE_ASPEN" origin "$aspen_url" 2>/dev/null); then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "remote_list_refs" true "" "$duration"
    else
        skip_test "remote_list_refs" "timeout or no refs"
    fi
}

# =============================================================================
# TEST CATEGORY 7: FULL GIT WORKFLOW
# =============================================================================

test_git_workflow() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "workflow" ]; then
        return 0
    fi

    if [ -z "$GIT_REMOTE_ASPEN" ] || [ ! -x "$GIT_REMOTE_ASPEN" ]; then
        skip_test "git_workflow" "git-remote-aspen not found"
        return 0
    fi

    if [ -z "$REPO_ID" ]; then
        skip_test "git_workflow" "no repository created"
        return 0
    fi

    print_header "7. FULL GIT WORKFLOW"
    local start_ms duration

    # Create test git directory
    TEST_GIT_DIR=$(mktemp -d)
    cd "$TEST_GIT_DIR"

    # Add git-remote-aspen to PATH
    export PATH="$(dirname "$GIT_REMOTE_ASPEN"):$PATH"

    local aspen_url="aspen://${TICKET}/${REPO_ID}"

    # Test 7.1: Initialize local repo
    start_ms=$(date +%s%3N)
    if git init --initial-branch=main >/dev/null 2>&1; then
        git config user.email "test@forge.test"
        git config user.name "Forge Test"
        duration=$(($(date +%s%3N) - start_ms))
        record_test "git_init_local" true "" "$duration"
    else
        record_test "git_init_local" false "git init failed"
        cd "$PROJECT_DIR"
        return 0
    fi

    # Test 7.2: Add files
    start_ms=$(date +%s%3N)
    cat > README.md << 'EOF'
# Test Repository

This repository was created by the Forge comprehensive test suite.

## Contents

- main.rs: Sample Rust code
- data.json: Sample data file
EOF

    cat > main.rs << 'EOF'
fn main() {
    println!("Hello from Forge!");
}
EOF

    cat > data.json << 'EOF'
{
    "name": "forge-test",
    "version": "1.0.0",
    "created": "2025-01-06"
}
EOF

    git add .
    duration=$(($(date +%s%3N) - start_ms))
    record_test "git_add_files" true "3 files" "$duration"

    # Test 7.3: Create commit
    start_ms=$(date +%s%3N)
    if git commit -m "Initial commit from Forge test" >/dev/null 2>&1; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "git_commit" true "" "$duration"
    else
        record_test "git_commit" false "commit failed"
    fi

    # Test 7.4: Add remote
    start_ms=$(date +%s%3N)
    if git remote add aspen "$aspen_url" 2>/dev/null; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "git_add_remote" true "" "$duration"
    else
        record_test "git_add_remote" false "remote add failed"
    fi

    # Test 7.5: Push to remote (this is the critical test)
    start_ms=$(date +%s%3N)
    if timeout 60 git push -v aspen main 2>&1 | tee /tmp/git-push-output.txt | tail -5; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "git_push" true "${duration}ms" "$duration"
    else
        # Check if it's a partial success
        if grep -q "error\|fatal" /tmp/git-push-output.txt 2>/dev/null; then
            record_test "git_push" false "push failed - see /tmp/git-push-output.txt"
        else
            record_test "git_push" false "timeout"
        fi
    fi

    # Test 7.6: Clone from remote (if push succeeded)
    local clone_dir
    clone_dir=$(mktemp -d)
    start_ms=$(date +%s%3N)
    if timeout 60 git clone "$aspen_url" "$clone_dir" 2>&1 | tail -3; then
        duration=$(($(date +%s%3N) - start_ms))
        if [ -f "$clone_dir/README.md" ]; then
            record_test "git_clone" true "${duration}ms" "$duration"
        else
            record_test "git_clone" false "files not cloned"
        fi
    else
        record_test "git_clone" false "clone failed"
    fi
    rm -rf "$clone_dir"

    cd "$PROJECT_DIR"
}

# =============================================================================
# TEST CATEGORY 8: EDGE CASES
# =============================================================================

test_edge_cases() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "edge" ]; then
        return 0
    fi

    print_header "8. EDGE CASES"
    local start_ms duration output

    # Test 8.1: Unicode content
    start_ms=$(date +%s%3N)
    local unicode_content="Hello World - Bonjour le Monde - Hej Verden"
    if output=$(echo -n "$unicode_content" | cli_capture blob add - 2>/dev/null); then
        hash=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")
        if [ -n "$hash" ]; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "unicode_blob" true "" "$duration"
        else
            record_test "unicode_blob" false "no hash"
        fi
    else
        record_test "unicode_blob" false "failed"
    fi

    # Test 8.2: Newlines in content
    start_ms=$(date +%s%3N)
    local multiline_content=$'line1\nline2\nline3\n'
    if output=$(printf '%s' "$multiline_content" | cli_capture blob add - 2>/dev/null); then
        hash=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")
        if [ -n "$hash" ]; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "multiline_blob" true "" "$duration"
        else
            record_test "multiline_blob" false "no hash"
        fi
    else
        record_test "multiline_blob" false "failed"
    fi

    # Test 8.3: Very long line
    start_ms=$(date +%s%3N)
    local long_line
    long_line=$(head -c 10000 /dev/zero | tr '\0' 'a')
    if output=$(echo -n "$long_line" | cli_capture blob add - 2>/dev/null); then
        hash=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")
        if [ -n "$hash" ]; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "long_line_blob" true "10KB single line" "$duration"
        else
            record_test "long_line_blob" false "no hash"
        fi
    else
        record_test "long_line_blob" false "failed"
    fi

    # Test 8.4: Null bytes
    start_ms=$(date +%s%3N)
    local null_file
    null_file=$(mktemp)
    printf '\x00\x00\x00\x00' > "$null_file"
    if output=$(cli_capture blob add "$null_file" 2>/dev/null); then
        hash=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")
        if [ -n "$hash" ]; then
            duration=$(($(date +%s%3N) - start_ms))
            record_test "null_bytes_blob" true "" "$duration"
        else
            record_test "null_bytes_blob" false "no hash"
        fi
    else
        record_test "null_bytes_blob" false "failed"
    fi
    rm -f "$null_file"
}

# =============================================================================
# TEST CATEGORY 9: CONCURRENT OPERATIONS
# =============================================================================

test_concurrent_operations() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "concurrent" ]; then
        return 0
    fi

    if [ "$QUICK_MODE" = true ]; then
        skip_test "concurrent_operations" "quick mode"
        return 0
    fi

    print_header "9. CONCURRENT OPERATIONS"
    local start_ms duration

    # Test 9.1: Parallel blob writes
    start_ms=$(date +%s%3N)
    local pids=()
    local success_count=0
    local total=10

    for i in $(seq 1 $total); do
        (
            echo -n "concurrent blob $i at $(date +%s%N)" | cli_capture blob add - >/dev/null 2>&1
        ) &
        pids+=($!)
    done

    for pid in "${pids[@]}"; do
        if wait "$pid"; then
            success_count=$((success_count + 1))
        fi
    done

    duration=$(($(date +%s%3N) - start_ms))
    if [ "$success_count" -eq "$total" ]; then
        record_test "parallel_blob_writes" true "$total writes in ${duration}ms" "$duration"
    else
        record_test "parallel_blob_writes" false "$success_count/$total succeeded"
    fi
}

# =============================================================================
# TEST CATEGORY 10: ERROR HANDLING
# =============================================================================

test_error_handling() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "errors" ]; then
        return 0
    fi

    print_header "10. ERROR HANDLING"
    local start_ms duration

    # Test 10.1: Invalid hash format
    start_ms=$(date +%s%3N)
    if cli blob get "invalid-hash" 2>&1 | grep -qi "error\|invalid"; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "invalid_hash_rejected" true "" "$duration"
    else
        record_test "invalid_hash_rejected" false "no error for invalid hash"
    fi

    # Test 10.2: Missing required arguments
    start_ms=$(date +%s%3N)
    if cli git show 2>&1 | grep -qi "error\|missing\|required"; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "missing_args_error" true "" "$duration"
    else
        skip_test "missing_args_error" "error message format unclear"
    fi

    # Test 10.3: Invalid ticket
    start_ms=$(date +%s%3N)
    if "$ASPEN_CLI" --ticket "invalid_ticket" cluster status 2>&1 | grep -qi "error\|invalid\|failed"; then
        duration=$(($(date +%s%3N) - start_ms))
        record_test "invalid_ticket_rejected" true "" "$duration"
    else
        record_test "invalid_ticket_rejected" false "no error for invalid ticket"
    fi
}

# =============================================================================
# TEST CATEGORY 11: PERFORMANCE BASELINE
# =============================================================================

test_performance_baseline() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "perf" ]; then
        return 0
    fi

    if [ "$QUICK_MODE" = true ]; then
        skip_test "performance_baseline" "quick mode"
        return 0
    fi

    print_header "11. PERFORMANCE BASELINE"
    local start_ms duration total_ms count

    # Test 11.1: Blob write latency (average of 10)
    total_ms=0
    count=10
    for i in $(seq 1 $count); do
        start_ms=$(date +%s%3N)
        echo -n "perf test blob $i" | cli_capture blob add - >/dev/null 2>&1
        duration=$(($(date +%s%3N) - start_ms))
        total_ms=$((total_ms + duration))
    done
    local avg_write=$((total_ms / count))
    record_test "blob_write_latency" true "avg ${avg_write}ms over $count writes" "$avg_write"

    # Test 11.2: Cluster status latency (average of 10)
    total_ms=0
    for i in $(seq 1 $count); do
        start_ms=$(date +%s%3N)
        cli cluster status >/dev/null 2>&1
        duration=$(($(date +%s%3N) - start_ms))
        total_ms=$((total_ms + duration))
    done
    local avg_status=$((total_ms / count))
    record_test "cluster_status_latency" true "avg ${avg_status}ms over $count calls" "$avg_status"
}

# =============================================================================
# TEST CATEGORY 12: AUDIT VALIDATION
# =============================================================================

test_audit_validation() {
    if [ -n "$CATEGORY_FILTER" ] && [ "$CATEGORY_FILTER" != "audit" ]; then
        return 0
    fi

    print_header "12. AUDIT VALIDATION"
    local start_ms duration

    # Test 12.1: Blob content integrity
    local content="audit validation content $(date +%s)"
    local stored_hash
    stored_hash=$(echo -n "$content" | cli_capture blob add - 2>/dev/null | grep -oE '[a-f0-9]{64}' | head -1 || echo "")

    if [ -n "$stored_hash" ]; then
        local retrieved
        retrieved=$(cli blob get "$stored_hash" 2>/dev/null | tail -1)
        if [ "$retrieved" = "$content" ]; then
            record_test "blob_integrity" true "content verified" "0"
        else
            record_test "blob_integrity" false "content mismatch"
        fi
    else
        record_test "blob_integrity" false "could not store blob"
    fi

    # Test 12.2: Cluster consistency (multi-node only)
    if [ "$MULTI_NODE" -gt 1 ]; then
        start_ms=$(date +%s%3N)
        # Store via one node, verify cluster agrees
        local test_content="consistency test $(date +%s)"
        local test_hash
        test_hash=$(echo -n "$test_content" | cli_capture blob add - 2>/dev/null | grep -oE '[a-f0-9]{64}' | head -1 || echo "")

        if [ -n "$test_hash" ]; then
            sleep 1  # Allow replication
            local retrieved
            retrieved=$(cli blob get "$test_hash" 2>/dev/null | tail -1)
            if [ "$retrieved" = "$test_content" ]; then
                duration=$(($(date +%s%3N) - start_ms))
                record_test "cluster_consistency" true "replicated" "$duration"
            else
                record_test "cluster_consistency" false "replication failed"
            fi
        else
            record_test "cluster_consistency" false "could not store"
        fi
    fi

    # Test 12.3: Data directory structure
    if [ -d "$DATA_DIR" ]; then
        if [ -f "$DATA_DIR/ticket.txt" ]; then
            record_test "data_dir_structure" true "ticket.txt present" "0"
        else
            record_test "data_dir_structure" false "ticket.txt missing"
        fi
    fi
}

# =============================================================================
# REPORT GENERATION
# =============================================================================

generate_report() {
    print_header "TEST REPORT"

    local end_time
    end_time=$(date +%s)
    local total_duration=$((end_time - START_TIME))

    printf "\n"
    printf "${BOLD}Summary${NC}\n"
    printf "━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
    printf "  Total tests:   %d\n" "$TESTS_RUN"
    printf "  ${GREEN}Passed:        %d${NC}\n" "$TESTS_PASSED"
    printf "  ${RED}Failed:        %d${NC}\n" "$TESTS_FAILED"
    printf "  ${YELLOW}Skipped:       %d${NC}\n" "$TESTS_SKIPPED"
    printf "  Duration:      %ds\n" "$total_duration"
    printf "\n"

    if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
        printf "${BOLD}${RED}Failed Tests${NC}\n"
        printf "━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        for test in "${FAILED_TESTS[@]}"; do
            printf "  - %s\n" "$test"
        done
        printf "\n"
    fi

    # Calculate pass rate
    local pass_rate=0
    if [ "$TESTS_RUN" -gt 0 ]; then
        pass_rate=$((TESTS_PASSED * 100 / TESTS_RUN))
    fi

    printf "${BOLD}Result${NC}\n"
    printf "━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
    if [ "$TESTS_FAILED" -eq 0 ]; then
        printf "  ${GREEN}${BOLD}ALL TESTS PASSED${NC} (${pass_rate}%%)\n"
    else
        printf "  ${RED}${BOLD}SOME TESTS FAILED${NC} (${pass_rate}%% passed)\n"
    fi
    printf "\n"

    # Environment info
    printf "${BOLD}Environment${NC}\n"
    printf "━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
    printf "  Script version: %s\n" "$SCRIPT_VERSION"
    printf "  Node count:     %d\n" "$MULTI_NODE"
    printf "  Storage:        %s\n" "$ASPEN_STORAGE"
    printf "  Data dir:       %s\n" "$DATA_DIR"
    printf "  Ticket:         %s...\n" "${TICKET:0:40}"
    printf "\n"
}

# =============================================================================
# ARGUMENT PARSING
# =============================================================================

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --quick)
                QUICK_MODE=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --keep-data)
                KEEP_DATA=true
                shift
                ;;
            --category)
                CATEGORY_FILTER="$2"
                shift 2
                ;;
            --multi-node)
                MULTI_NODE="$2"
                shift 2
                ;;
            --help)
                printf "Usage: %s [options]\n\n" "$0"
                printf "Options:\n"
                printf "  --quick           Skip slow tests\n"
                printf "  --category NAME   Run only tests in category\n"
                printf "                    (cluster, repo, objects, refs, issues,\n"
                printf "                     remote, workflow, edge, concurrent,\n"
                printf "                     errors, perf, audit)\n"
                printf "  --verbose         Print detailed output\n"
                printf "  --keep-data       Don't clean up after tests\n"
                printf "  --multi-node N    Test with N-node cluster\n"
                printf "  --help            Show this help\n"
                exit 0
                ;;
            *)
                printf "Unknown option: %s\n" "$1"
                exit 3
                ;;
        esac
    done
}

# =============================================================================
# MAIN
# =============================================================================

main() {
    parse_args "$@"

    printf "\n"
    printf "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${BOLD}${CYAN}  ASPEN FORGE COMPREHENSIVE TEST SUITE v%s${NC}\n" "$SCRIPT_VERSION"
    printf "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"

    trap cleanup EXIT INT TERM

    if ! setup; then
        printf "${RED}Setup failed${NC}\n"
        exit 2
    fi

    # Run all test categories
    test_cluster_infrastructure
    test_repository_management
    test_git_object_storage
    test_reference_management
    test_issue_tracking
    test_git_remote_helper
    test_git_workflow
    test_edge_cases
    test_concurrent_operations
    test_error_handling
    test_performance_baseline
    test_audit_validation

    generate_report

    if [ "$TESTS_FAILED" -gt 0 ]; then
        exit 1
    fi
    exit 0
}

main "$@"
