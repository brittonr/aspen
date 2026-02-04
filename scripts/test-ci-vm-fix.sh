#!/usr/bin/env bash
# Test CI VM Worker Registration Fix
#
# This script verifies the fix that prevents EchoWorker from catching ci_vm
# and ci_nix_build jobs. These job types should wait in the queue until their
# specialized workers (LocalExecutorWorker/CloudHypervisorWorker/NixBuildWorker)
# register.
#
# The fix is in:
#   - crates/aspen-jobs/src/workers/echo.rs:10-15 - EXCLUDED_JOB_TYPES constant
#   - crates/aspen-jobs/src/worker.rs:341-366 - Handler lookup with can_handle()
#
# Usage:
#   ./scripts/test-ci-vm-fix.sh           # Run full test
#   ./scripts/test-ci-vm-fix.sh --quick   # Skip build verification
#
# Environment variables:
#   ASPEN_DATA_DIR    - Data directory (default: /tmp/aspen-ci-vm-fix-test)
#   ASPEN_LOG_LEVEL   - Log level (default: info,aspen_jobs=debug)
#   ASPEN_SKIP_BUILD  - Skip build verification (default: false)

set -eu

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="${PROJECT_DIR:-$(cd "$SCRIPT_DIR/.." && pwd)}"

# Source shared functions (provides find_binary, wait_for_*, etc.)
. "$SCRIPT_DIR/lib/cluster-common.sh"

# Configuration
DATA_DIR="${ASPEN_DATA_DIR:-/tmp/aspen-ci-vm-fix-test}"
LOG_LEVEL="${ASPEN_LOG_LEVEL:-info,aspen_jobs=debug,aspen_ci=debug}"
COOKIE="ci-vm-fix-test-$(date +%s)"
SKIP_BUILD="${ASPEN_SKIP_BUILD:-false}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Track results
TESTS_PASSED=0
TESTS_FAILED=0
declare -a FAILED_TESTS

# Node PID for cleanup
NODE_PID=""

# Cleanup function
cleanup() {
    printf "\n${BLUE}Cleaning up...${NC}\n"

    if [ -n "$NODE_PID" ] && kill -0 "$NODE_PID" 2>/dev/null; then
        printf "  Stopping node (PID %s)..." "$NODE_PID"
        kill "$NODE_PID" 2>/dev/null || true
        sleep 0.5
        if kill -0 "$NODE_PID" 2>/dev/null; then
            kill -9 "$NODE_PID" 2>/dev/null || true
        fi
        printf " done\n"
    fi

    printf "${GREEN}Cleanup complete${NC}\n"
}

trap cleanup EXIT INT TERM

# Print test result
print_result() {
    local name="$1"
    local passed="$2"
    local msg="${3:-}"

    if [ "$passed" = "true" ]; then
        printf "  ${GREEN}PASS${NC} %s\n" "$name"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        printf "  ${RED}FAIL${NC} %s" "$name"
        if [ -n "$msg" ]; then
            printf " - %s" "$msg"
        fi
        printf "\n"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        FAILED_TESTS+=("$name")
    fi
}

# Phase 0: Build Verification
verify_build() {
    printf "${BLUE}Phase 0: Build Verification${NC}\n"

    if [ "$SKIP_BUILD" = "true" ]; then
        printf "  Skipping build verification (ASPEN_SKIP_BUILD=true)\n\n"
        return 0
    fi

    # Verify fix files exist and contain expected content
    printf "  Checking fix files...\n"

    local echo_rs="$PROJECT_DIR/crates/aspen-jobs/src/workers/echo.rs"
    local worker_rs="$PROJECT_DIR/crates/aspen-jobs/src/worker.rs"

    if [ ! -f "$echo_rs" ]; then
        print_result "echo.rs exists" "false" "File not found: $echo_rs"
        return 1
    fi

    # Check EXCLUDED_JOB_TYPES contains ci_vm
    if ! grep -q 'ci_vm' "$echo_rs"; then
        print_result "EXCLUDED_JOB_TYPES contains ci_vm" "false"
        return 1
    fi
    print_result "EXCLUDED_JOB_TYPES contains ci_vm" "true"

    # Check EXCLUDED_JOB_TYPES contains ci_nix_build
    if ! grep -q 'ci_nix_build' "$echo_rs"; then
        print_result "EXCLUDED_JOB_TYPES contains ci_nix_build" "false"
        return 1
    fi
    print_result "EXCLUDED_JOB_TYPES contains ci_nix_build" "true"

    # Check worker.rs has can_handle check
    if ! grep -q 'can_handle' "$worker_rs"; then
        print_result "worker.rs uses can_handle()" "false"
        return 1
    fi
    print_result "worker.rs uses can_handle()" "true"

    # Build with required features
    # Note: aspen-node is part of the main 'aspen' package, aspen-cli is in 'aspen-cli' package
    printf "  Building with CI features...\n"
    if ! cargo build -p aspen --features "ci,forge,git-bridge,shell-worker,blob" 2>&1; then
        print_result "cargo build aspen-node" "false" "Build failed"
        return 1
    fi
    print_result "cargo build aspen-node" "true"

    if ! cargo build -p aspen-cli 2>&1; then
        print_result "cargo build aspen-cli" "false" "Build failed"
        return 1
    fi
    print_result "cargo build aspen-cli" "true"

    printf "\n"
    return 0
}

# Phase 1: Setup Cluster
setup_cluster() {
    printf "${BLUE}Phase 1: Setup LocalExecutorWorker Cluster${NC}\n"

    # Find binaries
    ASPEN_NODE_BIN="${ASPEN_NODE_BIN:-$(find_binary aspen-node)}"
    ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"

    if [ -z "$ASPEN_NODE_BIN" ] || [ ! -x "$ASPEN_NODE_BIN" ]; then
        print_result "aspen-node binary found" "false"
        return 1
    fi
    print_result "aspen-node binary found" "true"

    if [ -z "$ASPEN_CLI_BIN" ] || [ ! -x "$ASPEN_CLI_BIN" ]; then
        print_result "aspen-cli binary found" "false"
        return 1
    fi
    print_result "aspen-cli binary found" "true"

    # Clean and create data directory
    rm -rf "$DATA_DIR"
    mkdir -p "$DATA_DIR/workspace"

    # Start node with LocalExecutorWorker
    printf "  Starting node with LocalExecutorWorker...\n"

    RUST_LOG="$LOG_LEVEL" \
    ASPEN_BLOBS_ENABLED="true" \
    ASPEN_CI_ENABLED="true" \
    ASPEN_CI_LOCAL_EXECUTOR="1" \
    ASPEN_CI_WORKSPACE_DIR="$DATA_DIR/workspace" \
    "$ASPEN_NODE_BIN" \
        --node-id 1 \
        --cookie "$COOKIE" \
        --data-dir "$DATA_DIR/node1" \
        --storage-backend inmemory \
        --enable-workers \
        --worker-count 2 \
        --enable-ci \
        > "$DATA_DIR/node.log" 2>&1 &

    NODE_PID=$!
    echo "$NODE_PID" > "$DATA_DIR/node.pid"
    printf "  Node started (PID %s)\n" "$NODE_PID"

    # Wait for ticket
    printf "  Waiting for ticket..."
    local timeout=30
    local elapsed=0
    TICKET=""

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$DATA_DIR/node.log" ]; then
            TICKET=$(grep -oE 'aspen[a-z2-7]{50,500}' "$DATA_DIR/node.log" 2>/dev/null | head -1 || true)
            if [ -n "$TICKET" ]; then
                printf " ${GREEN}done${NC}\n"
                break
            fi
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done

    if [ -z "$TICKET" ]; then
        print_result "Get ticket" "false" "Timeout after ${timeout}s"
        return 1
    fi
    print_result "Get ticket" "true"
    echo "$TICKET" > "$DATA_DIR/ticket.txt"

    # Initialize cluster
    printf "  Initializing cluster..."
    if ! "$ASPEN_CLI_BIN" --quiet --ticket "$TICKET" cluster init >/dev/null 2>&1; then
        printf " ${RED}failed${NC}\n"
        print_result "Cluster init" "false"
        return 1
    fi
    printf " ${GREEN}done${NC}\n"
    print_result "Cluster init" "true"

    # Wait for cluster to stabilize
    printf "  Waiting for cluster to stabilize..."
    if wait_for_cluster_stable "$ASPEN_CLI_BIN" "$TICKET" 30000 30; then
        printf " ${GREEN}done${NC}\n"
        print_result "Cluster stable" "true"
    else
        printf " ${YELLOW}timeout${NC}\n"
        print_result "Cluster stable" "false" "Timeout"
        # Continue anyway - cluster may still work
    fi

    # Wait for job subsystem
    printf "  Waiting for job subsystem..."
    if wait_for_subsystem "$ASPEN_CLI_BIN" "$TICKET" 30000 job 30; then
        printf " ${GREEN}done${NC}\n"
        print_result "Job subsystem ready" "true"
    else
        printf " ${RED}failed${NC}\n"
        print_result "Job subsystem ready" "false"
        return 1
    fi

    # Check workers are registered
    printf "  Checking worker registration...\n"
    local workers_output
    workers_output=$("$ASPEN_CLI_BIN" --ticket "$TICKET" job workers 2>&1 || true)

    if echo "$workers_output" | grep -qi "total.*workers.*[1-9]"; then
        print_result "Workers registered" "true"
    else
        print_result "Workers registered" "false" "No workers found"
        printf "  Workers output: %s\n" "$workers_output"
    fi

    # Check LocalExecutorWorker specifically in logs
    if grep -qi "Local.*Executor.*worker.*registered\|LocalExecutorWorker" "$DATA_DIR/node.log" 2>/dev/null; then
        print_result "LocalExecutorWorker in logs" "true"
    else
        # It might be named differently, check for CI executor
        if grep -qi "ci.*local.*executor\|local.*executor.*ci" "$DATA_DIR/node.log" 2>/dev/null; then
            print_result "LocalExecutorWorker in logs" "true"
        else
            print_result "LocalExecutorWorker in logs" "false" "Not found in logs (may still work)"
        fi
    fi

    printf "\n"
    return 0
}

# Phase 2: Test ci_vm Job Type
test_ci_vm_job() {
    printf "${BLUE}Phase 2: Test ci_vm Job Type${NC}\n"

    # Submit a ci_vm job
    local payload='{"type":"ci_vm","command":"echo","args":["hello from ci_vm"],"timeout_secs":60}'

    printf "  Submitting ci_vm job...\n"
    local submit_output
    submit_output=$("$ASPEN_CLI_BIN" --ticket "$TICKET" job submit ci_vm "$payload" 2>&1 || true)

    local job_id
    job_id=$(echo "$submit_output" | grep -oE '[a-f0-9-]{36}' | head -1 || true)

    if [ -z "$job_id" ]; then
        # Job might be pending (no specialized worker)
        # This is actually the EXPECTED behavior if LocalExecutorWorker isn't handling ci_vm
        if echo "$submit_output" | grep -qi "submitted\|job_id"; then
            job_id=$(echo "$submit_output" | grep -oE '[a-f0-9-]{30,}' | head -1 || true)
        fi
    fi

    if [ -n "$job_id" ]; then
        print_result "ci_vm job submitted" "true"
        printf "  Job ID: %s\n" "$job_id"
    else
        print_result "ci_vm job submitted" "false" "Could not extract job ID"
        printf "  Output: %s\n" "$submit_output"
        # Don't fail - the key test is log verification
    fi

    # Wait a moment for job processing
    sleep 3

    # Check job status
    if [ -n "$job_id" ]; then
        local status_output
        status_output=$("$ASPEN_CLI_BIN" --ticket "$TICKET" job get "$job_id" 2>&1 || true)
        printf "  Job status: %s\n" "$(echo "$status_output" | grep -i status | head -1 || echo "unknown")"
    fi

    # The key verification: EchoWorker should NOT process ci_vm jobs
    printf "  Checking logs for EchoWorker handling ci_vm...\n"

    local echo_handled_ci_vm="false"
    if grep -i "echo.*worker.*processing.*job" "$DATA_DIR/node.log" 2>/dev/null | grep -qi "ci_vm"; then
        echo_handled_ci_vm="true"
    fi

    if [ "$echo_handled_ci_vm" = "false" ]; then
        print_result "EchoWorker did NOT handle ci_vm" "true"
    else
        print_result "EchoWorker did NOT handle ci_vm" "false" "EchoWorker processed ci_vm job!"
    fi

    printf "\n"
    return 0
}

# Phase 3: Test ci_nix_build Job Type
test_ci_nix_build_job() {
    printf "${BLUE}Phase 3: Test ci_nix_build Job Type${NC}\n"

    # Submit a ci_nix_build job
    local payload='{"type":"ci_nix_build","flake_url":".","flake_attr":"packages.x86_64-linux.hello","timeout_secs":120}'

    printf "  Submitting ci_nix_build job...\n"
    local submit_output
    submit_output=$("$ASPEN_CLI_BIN" --ticket "$TICKET" job submit ci_nix_build "$payload" 2>&1 || true)

    local job_id
    job_id=$(echo "$submit_output" | grep -oE '[a-f0-9-]{36}' | head -1 || true)

    if [ -z "$job_id" ]; then
        if echo "$submit_output" | grep -qi "submitted\|job_id"; then
            job_id=$(echo "$submit_output" | grep -oE '[a-f0-9-]{30,}' | head -1 || true)
        fi
    fi

    if [ -n "$job_id" ]; then
        print_result "ci_nix_build job submitted" "true"
        printf "  Job ID: %s\n" "$job_id"
    else
        print_result "ci_nix_build job submitted" "false" "Could not extract job ID"
        printf "  Output: %s\n" "$submit_output"
    fi

    # Wait a moment for job processing
    sleep 3

    # Check job status
    if [ -n "$job_id" ]; then
        local status_output
        status_output=$("$ASPEN_CLI_BIN" --ticket "$TICKET" job get "$job_id" 2>&1 || true)
        printf "  Job status: %s\n" "$(echo "$status_output" | grep -i status | head -1 || echo "unknown")"
    fi

    # The key verification: EchoWorker should NOT process ci_nix_build jobs
    printf "  Checking logs for EchoWorker handling ci_nix_build...\n"

    local echo_handled_ci_nix="false"
    if grep -i "echo.*worker.*processing.*job" "$DATA_DIR/node.log" 2>/dev/null | grep -qi "ci_nix_build"; then
        echo_handled_ci_nix="true"
    fi

    if [ "$echo_handled_ci_nix" = "false" ]; then
        print_result "EchoWorker did NOT handle ci_nix_build" "true"
    else
        print_result "EchoWorker did NOT handle ci_nix_build" "false" "EchoWorker processed ci_nix_build job!"
    fi

    printf "\n"
    return 0
}

# Phase 4: Test that shell_command jobs ARE handled
test_shell_command_job() {
    printf "${BLUE}Phase 4: Test shell_command Job Type (should be handled)${NC}\n"

    # Submit a shell_command job - this SHOULD be handled by EchoWorker or ShellCommandWorker
    local payload='{"command":"echo","args":["hello from shell"]}'

    printf "  Submitting shell_command job...\n"
    local submit_output
    submit_output=$("$ASPEN_CLI_BIN" --ticket "$TICKET" job submit shell_command "$payload" 2>&1 || true)

    local job_id
    job_id=$(echo "$submit_output" | grep -oE '[a-f0-9-]{36}' | head -1 || true)

    if [ -n "$job_id" ]; then
        print_result "shell_command job submitted" "true"
        printf "  Job ID: %s\n" "$job_id"

        # Wait for job to complete
        sleep 3

        local status_output
        status_output=$("$ASPEN_CLI_BIN" --ticket "$TICKET" job get "$job_id" 2>&1 || true)

        if echo "$status_output" | grep -qi "completed\|success"; then
            print_result "shell_command job completed" "true"
        else
            printf "  Status: %s\n" "$(echo "$status_output" | grep -i status | head -1 || echo "unknown")"
            print_result "shell_command job completed" "false" "Job did not complete"
        fi
    else
        print_result "shell_command job submitted" "false"
    fi

    printf "\n"
    return 0
}

# Phase 5: Comprehensive Log Verification
verify_logs() {
    printf "${BLUE}Phase 5: Comprehensive Log Verification${NC}\n"

    local log_file="$DATA_DIR/node.log"

    if [ ! -f "$log_file" ]; then
        print_result "Log file exists" "false"
        return 1
    fi
    print_result "Log file exists" "true"

    # Count echo worker processing messages
    local echo_processing_count
    echo_processing_count=$(grep -c "echo.*worker.*processing" "$log_file" 2>/dev/null || echo "0")
    printf "  EchoWorker processing count: %s\n" "$echo_processing_count"

    # Check if any echo worker processing was for excluded types
    local echo_bad_processing="false"
    if grep "echo.*worker.*processing" "$log_file" 2>/dev/null | grep -qiE "ci_vm|ci_nix_build"; then
        echo_bad_processing="true"
    fi

    if [ "$echo_bad_processing" = "false" ]; then
        print_result "No excluded jobs processed by EchoWorker" "true"
    else
        print_result "No excluded jobs processed by EchoWorker" "false"
        printf "  ${RED}EchoWorker incorrectly processed ci_vm or ci_nix_build jobs!${NC}\n"
    fi

    # Check for can_handle debug messages
    if grep -qi "can_handle\|job_types" "$log_file" 2>/dev/null; then
        print_result "Handler selection logic in logs" "true"
    else
        print_result "Handler selection logic in logs" "false" "No can_handle logs (debug level may be needed)"
    fi

    printf "\n"
}

# Phase 6: SNIX Verification (optional - may not be configured)
verify_snix() {
    printf "${BLUE}Phase 6: SNIX Cache Verification (Optional)${NC}\n"

    # Check cache stats - this verifies the cache subsystem is available
    local stats_output
    stats_output=$("$ASPEN_CLI_BIN" --ticket "$TICKET" cache stats 2>&1 || true)

    if echo "$stats_output" | grep -qi "error\|not.*found\|unavailable"; then
        printf "  SNIX cache not available (expected in test mode)\n"
        print_result "SNIX cache stats" "false" "Not configured (expected)"
    else
        print_result "SNIX cache stats" "true"
        printf "  Stats: %s\n" "$(echo "$stats_output" | head -5)"
    fi

    printf "\n"
}

# Print final summary
print_summary() {
    printf "${BLUE}======================================${NC}\n"
    printf "${BLUE}Test Summary${NC}\n"
    printf "${BLUE}======================================${NC}\n"

    printf "  ${GREEN}Passed${NC}: %d\n" "$TESTS_PASSED"
    printf "  ${RED}Failed${NC}: %d\n" "$TESTS_FAILED"

    if [ "$TESTS_FAILED" -gt 0 ]; then
        printf "\n  ${RED}Failed tests:${NC}\n"
        for test in "${FAILED_TESTS[@]}"; do
            printf "    - %s\n" "$test"
        done
    fi

    printf "\n"

    if [ "$TESTS_FAILED" -eq 0 ]; then
        printf "${GREEN}All tests passed!${NC}\n"
        printf "\nThe fix is working correctly:\n"
        printf "  - EchoWorker excludes ci_vm and ci_nix_build jobs\n"
        printf "  - These jobs wait in queue for specialized workers\n"
        printf "  - Other job types (shell_command) are still handled\n"
        return 0
    else
        printf "${RED}Some tests failed.${NC}\n"
        printf "\nCheck logs at: %s/node.log\n" "$DATA_DIR"
        return 1
    fi
}

# Main entry point
main() {
    # Parse arguments
    for arg in "$@"; do
        case "$arg" in
            --quick)
                SKIP_BUILD="true"
                ;;
            --help|-h)
                printf "Usage: %s [--quick]\n" "$0"
                printf "\nOptions:\n"
                printf "  --quick    Skip build verification\n"
                printf "\nEnvironment:\n"
                printf "  ASPEN_DATA_DIR    Data directory (default: /tmp/aspen-ci-vm-fix-test)\n"
                printf "  ASPEN_LOG_LEVEL   Log level (default: info,aspen_jobs=debug)\n"
                printf "  ASPEN_SKIP_BUILD  Skip build verification (default: false)\n"
                exit 0
                ;;
        esac
    done

    printf "${BLUE}============================================${NC}\n"
    printf "${GREEN}CI VM Worker Registration Fix Test${NC}\n"
    printf "${BLUE}============================================${NC}\n\n"
    printf "Data directory: %s\n" "$DATA_DIR"
    printf "Skip build: %s\n\n" "$SKIP_BUILD"

    # Run test phases
    verify_build || true
    setup_cluster || exit 1
    test_ci_vm_job
    test_ci_nix_build_job
    test_shell_command_job
    verify_logs
    verify_snix

    # Print summary and exit with appropriate code
    print_summary
}

main "$@"
