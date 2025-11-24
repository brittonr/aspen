#!/usr/bin/env bash
# Integration test runner with flawless server management
# This script ensures all required services are running before tests

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Cleanup function
cleanup() {
    local exit_code=$?

    log_info "Cleaning up..."

    # Kill flawless server if we started it
    if [[ -n "${FLAWLESS_PID:-}" ]]; then
        log_info "Stopping flawless server (PID: $FLAWLESS_PID)"
        kill "$FLAWLESS_PID" 2>/dev/null || true
        wait "$FLAWLESS_PID" 2>/dev/null || true
    fi

    # Clean up temporary files
    rm -f /tmp/flawless-test-*.log

    if [[ $exit_code -eq 0 ]]; then
        log_info "Integration tests completed successfully ✓"
    else
        log_error "Integration tests failed with exit code $exit_code"
    fi

    exit $exit_code
}

# Register cleanup on exit
trap cleanup EXIT INT TERM

# Check if flawless is in PATH
if ! command -v flawless &> /dev/null; then
    log_error "flawless command not found in PATH"
    log_error "Make sure you're running this in nix develop or nix run"
    exit 1
fi

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    log_error "cargo command not found in PATH"
    exit 1
fi

log_info "Starting flawless server..."

# Start flawless server in background
flawless up > /tmp/flawless-test-$$.log 2>&1 &
FLAWLESS_PID=$!

log_info "Flawless server started with PID: $FLAWLESS_PID"

# Wait for flawless server to be ready (up to 10 seconds)
log_info "Waiting for flawless server to be ready..."
for i in {1..20}; do
    if curl -s http://localhost:27288/health > /dev/null 2>&1; then
        log_info "Flawless server is ready!"
        break
    fi

    if [[ $i -eq 20 ]]; then
        log_error "Flawless server failed to start within 10 seconds"
        log_error "Server logs:"
        cat /tmp/flawless-test-$$.log
        exit 1
    fi

    # Check if process is still running
    if ! kill -0 "$FLAWLESS_PID" 2>/dev/null; then
        log_error "Flawless server process died"
        log_error "Server logs:"
        cat /tmp/flawless-test-$$.log
        exit 1
    fi

    sleep 0.5
done

# Set environment variables for tests
export FLAWLESS_URL="http://localhost:27288"
export RUST_LOG="${RUST_LOG:-info}"

log_info "Building test project..."
cargo build --tests

log_info "Running integration tests..."
log_info "----------------------------------------"

# Run the integration tests
cargo test --test scenario_tests -- --nocapture --test-threads=1

log_info "----------------------------------------"
log_info "All integration tests passed! ✓"
