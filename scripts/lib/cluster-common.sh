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