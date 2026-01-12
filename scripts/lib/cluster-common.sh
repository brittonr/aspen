#!/bin/bash
# Shared functions for cluster launch scripts
# Source this file in cluster.sh and kitty-cluster.sh

# Find binary in various locations
find_binary() {
    local name="$1"
    local bin=""

    # Check environment variable first
    local env_var="ASPEN_${name^^}_BIN"
    env_var="${env_var//-/_}"
    bin="${!env_var:-}"
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check PATH
    bin=$(command -v "$name" 2>/dev/null || echo "")
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check target/release
    bin="$PROJECT_DIR/target/release/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check target/debug
    bin="$PROJECT_DIR/target/debug/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check nix result symlink
    bin="$PROJECT_DIR/result/bin/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    echo ""
}

# Generate deterministic secret key for a node
generate_secret_key() {
    local node_id="$1"
    printf '%064x' "$((1000 + node_id))"
}

# Check prerequisites
check_prerequisites() {
    if [ -z "$ASPEN_NODE_BIN" ] || [ ! -x "$ASPEN_NODE_BIN" ]; then
        printf "${RED}Error: aspen-node binary not found${NC}\n" >&2
        printf "Build with: cargo build --release --bin aspen-node\n" >&2
        printf "Or use: nix run .#cluster\n" >&2
        exit 1
    fi

    if [ -z "$ASPEN_CLI_BIN" ] || [ ! -x "$ASPEN_CLI_BIN" ]; then
        printf "${RED}Error: aspen-cli binary not found${NC}\n" >&2
        printf "Build with: cargo build --release --bin aspen-cli\n" >&2
        exit 1
    fi
}

# =============================================================================
# SHARED TEST UTILITIES
# =============================================================================

# Retry a command with exponential backoff
# Usage: retry_command [max_attempts] [initial_delay] command [args...]
retry_command() {
    local max_attempts="${1:-3}"
    local delay="${2:-1}"
    shift 2
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if "$@"; then
            return 0
        fi
        if [ $attempt -lt $max_attempts ]; then
            sleep $delay
            delay=$((delay * 2))
        fi
        attempt=$((attempt + 1))
    done
    return 1
}

# Wait for a condition to become true
# Usage: wait_for_condition [timeout_secs] [poll_interval] command [args...]
wait_for_condition() {
    local timeout="${1:-30}"
    local interval="${2:-1}"
    shift 2
    local elapsed=0

    while [ "$elapsed" -lt "$timeout" ]; do
        if "$@" >/dev/null 2>&1; then
            return 0
        fi
        sleep "$interval"
        elapsed=$((elapsed + interval))
    done
    return 1
}

# Extract a field from JSON output
# Usage: extract_json_field "field_name" "json_string"
extract_json_field() {
    local field="$1"
    local json="$2"

    # Try to extract using grep/sed (no jq dependency)
    echo "$json" | grep -oE "\"$field\":\s*\"[^\"]+\"" | sed "s/\"$field\":\s*\"//" | sed 's/"$//' | head -1 || true
}

# Extract a numeric field from JSON output
# Usage: extract_json_number "field_name" "json_string"
extract_json_number() {
    local field="$1"
    local json="$2"

    echo "$json" | grep -oE "\"$field\":\s*[0-9]+" | grep -oE '[0-9]+' | head -1 || true
}

# Extract a 64-char hex hash from output
# Usage: extract_hash "output_string"
extract_hash() {
    local output="$1"
    echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || true
}
