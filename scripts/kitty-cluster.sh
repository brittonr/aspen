#!/usr/bin/env bash
# Kitty terminal cluster launcher for Aspen
# Builds the project and spawns nodes in a multi-pane kitty layout
#
# Usage: ./scripts/kitty-cluster.sh [options]
#
# Options:
#   --nodes N       Number of nodes (default: 3)
#   --release       Build in release mode (default: debug)
#   --no-build      Skip the build step
#   --features F    Additional cargo features (comma-separated)
#   --workers       Enable job workers
#   --ci            Enable CI system
#   --blobs         Enable blob storage
#   --docs          Enable iroh-docs
#   --help          Show this help
#
# Environment variables:
#   ASPEN_NODE_COUNT  - Number of nodes (default: 3)
#   ASPEN_COOKIE      - Cluster cookie (default: kitty-cluster-PID)
#   ASPEN_LOG_LEVEL   - Log level (default: info)
#   ASPEN_DATA_DIR    - Data directory (default: /tmp/aspen-kitty-$$)

set -euo pipefail

# Defaults
NODE_COUNT="${ASPEN_NODE_COUNT:-3}"
COOKIE="${ASPEN_COOKIE:-kitty-cluster-$$}"
LOG_LEVEL="${ASPEN_LOG_LEVEL:-info}"
DATA_DIR="${ASPEN_DATA_DIR:-/tmp/aspen-kitty-$$}"
BUILD_MODE="debug"
SKIP_BUILD=false
EXTRA_FEATURES=""
WORKERS_ENABLED=false
CI_ENABLED=false
BLOBS_ENABLED=false
DOCS_ENABLED=false

# Resolve directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --nodes)
            NODE_COUNT="$2"
            shift 2
            ;;
        --release)
            BUILD_MODE="release"
            shift
            ;;
        --no-build)
            SKIP_BUILD=true
            shift
            ;;
        --features)
            EXTRA_FEATURES="$2"
            shift 2
            ;;
        --workers)
            WORKERS_ENABLED=true
            shift
            ;;
        --ci)
            CI_ENABLED=true
            shift
            ;;
        --blobs)
            BLOBS_ENABLED=true
            shift
            ;;
        --docs)
            DOCS_ENABLED=true
            shift
            ;;
        --help|-h)
            printf "Kitty Cluster Launcher for Aspen\n\n"
            printf "Usage: %s [options]\n\n" "$0"
            printf "Options:\n"
            printf "  --nodes N       Number of nodes (default: 3)\n"
            printf "  --release       Build in release mode\n"
            printf "  --no-build      Skip the build step\n"
            printf "  --features F    Additional cargo features\n"
            printf "  --workers       Enable job workers\n"
            printf "  --ci            Enable CI system\n"
            printf "  --blobs         Enable blob storage\n"
            printf "  --docs          Enable iroh-docs\n"
            printf "  --help          Show this help\n"
            exit 0
            ;;
        *)
            printf "${RED}Unknown option: %s${NC}\n" "$1"
            exit 1
            ;;
    esac
done

# Check for kitty
if ! command -v kitty &>/dev/null; then
    printf "${RED}Error: kitty terminal not found${NC}\n"
    printf "Install kitty: https://sw.kovidgoyal.net/kitty/\n"
    exit 1
fi

# Source common functions
source "$SCRIPT_DIR/lib/cluster-common.sh"

# Generate deterministic secret key
generate_secret_key() {
    local node_id="$1"
    printf '%064x' "$((1000 + node_id))"
}

# Build the project
build_project() {
    printf "${BLUE}Building Aspen...${NC}\n\n"

    local release_flag=""
    if [ "$BUILD_MODE" = "release" ]; then
        release_flag="--release"
    fi

    # Build features list
    local features="forge,git-bridge,secrets"
    if [ "$CI_ENABLED" = true ]; then
        features="$features,ci"
    fi
    if [ -n "$EXTRA_FEATURES" ]; then
        features="$features,$EXTRA_FEATURES"
    fi

    cd "$PROJECT_DIR"

    # Build aspen-node (main crate)
    printf "  Building aspen-node...\n"
    if ! cargo build $release_flag --features "$features" --bin aspen-node; then
        printf "\n${RED}Build failed (aspen-node)!${NC}\n"
        exit 1
    fi

    # Build aspen-cli (separate crate)
    printf "  Building aspen-cli...\n"
    if ! cargo build $release_flag -p aspen-cli --features ci; then
        printf "\n${RED}Build failed (aspen-cli)!${NC}\n"
        exit 1
    fi

    printf "\n${GREEN}Build complete!${NC}\n\n"
}

# Get binary path
get_binary() {
    local name="$1"
    if [ "$BUILD_MODE" = "release" ]; then
        echo "$PROJECT_DIR/target/release/$name"
    else
        echo "$PROJECT_DIR/target/debug/$name"
    fi
}

# Start a node in the background
start_node() {
    local node_id="$1"
    local node_bin="$2"

    local node_data_dir="$DATA_DIR/node$node_id"
    local secret_key
    secret_key=$(generate_secret_key "$node_id")
    local log_file="$node_data_dir/node.log"

    mkdir -p "$node_data_dir"

    # Build node arguments
    local node_args=(
        "--node-id" "$node_id"
        "--cookie" "$COOKIE"
        "--data-dir" "$node_data_dir"
        "--storage-backend" "inmemory"
        "--iroh-secret-key" "$secret_key"
    )

    if [ "$WORKERS_ENABLED" = true ]; then
        node_args+=("--enable-workers" "--worker-count" "4")
    fi

    if [ "$CI_ENABLED" = true ]; then
        node_args+=("--enable-ci")
    fi

    # Start the node
    RUST_LOG="$LOG_LEVEL" \
    ASPEN_BLOBS_ENABLED="$BLOBS_ENABLED" \
    ASPEN_DOCS_ENABLED="$DOCS_ENABLED" \
    "$node_bin" "${node_args[@]}" > "$log_file" 2>&1 &

    local pid=$!
    echo "$pid" >> "$DATA_DIR/pids"
    printf "  Node %d: PID %d (log: %s)\n" "$node_id" "$pid" "$log_file"
}

# Wait for ticket from node 1
wait_for_ticket() {
    local log_file="$DATA_DIR/node1/node.log"
    local timeout=60
    local elapsed=0

    printf "  Waiting for node 1 to start"

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$log_file" ]; then
            local ticket
            ticket=$(grep -oE 'aspen[a-z2-7]{50,200}' "$log_file" 2>/dev/null | head -1 || true)

            if [ -n "$ticket" ]; then
                printf " ${GREEN}done${NC}\n"
                echo "$ticket" > "$DATA_DIR/ticket.txt"
                echo "$ticket"
                return 0
            fi
        fi

        printf "."
        sleep 1
        elapsed=$((elapsed + 1))
    done

    printf " ${RED}timeout${NC}\n"
    return 1
}

# Get endpoint ID from log
get_endpoint_id() {
    local log_file="$1"
    sed 's/\x1b\[[0-9;]*m//g' "$log_file" 2>/dev/null | \
        grep -oE 'endpoint_id=[a-f0-9]{64}' | head -1 | cut -d= -f2 || true
}

# Initialize cluster via CLI
init_cluster() {
    local cli_bin="$1"
    local ticket="$2"

    printf "  Waiting for gossip discovery..."
    sleep 5
    printf " ${GREEN}done${NC}\n"

    # Initialize node 1
    printf "  Initializing cluster..."
    local attempts=0
    while [ "$attempts" -lt 10 ]; do
        if "$cli_bin" --ticket "$ticket" cluster init >/dev/null 2>&1; then
            printf " ${GREEN}done${NC}\n"
            break
        fi
        attempts=$((attempts + 1))
        printf "."
        sleep 2
    done

    if [ "$attempts" -eq 10 ]; then
        printf " ${RED}failed${NC}\n"
        return 1
    fi

    sleep 2

    # Add other nodes
    if [ "$NODE_COUNT" -gt 1 ]; then
        printf "  Adding nodes 2-%d as learners...\n" "$NODE_COUNT"

        local id=2
        while [ "$id" -le "$NODE_COUNT" ]; do
            local endpoint_id
            endpoint_id=$(get_endpoint_id "$DATA_DIR/node$id/node.log")

            if [ -n "$endpoint_id" ]; then
                printf "    Node %d: " "$id"
                if "$cli_bin" --ticket "$ticket" cluster add-learner \
                    --node-id "$id" --addr "$endpoint_id" >/dev/null 2>&1; then
                    printf "${GREEN}added${NC}\n"
                else
                    printf "${YELLOW}skipped${NC}\n"
                fi
            fi
            sleep 1
            id=$((id + 1))
        done

        # Promote to voters
        printf "  Promoting to voters..."
        local members=""
        id=1
        while [ "$id" -le "$NODE_COUNT" ]; do
            if [ -n "$members" ]; then
                members="$members $id"
            else
                members="$id"
            fi
            id=$((id + 1))
        done

        sleep 2
        if "$cli_bin" --ticket "$ticket" cluster change-membership $members >/dev/null 2>&1; then
            printf " ${GREEN}done${NC}\n"
        else
            printf " ${YELLOW}skipped${NC}\n"
        fi
    fi
}

# Open kitty layout with log windows
open_kitty_layout() {
    local ticket="$1"
    local cli_bin="$2"

    printf "\n${BLUE}Opening kitty layout...${NC}\n"

    # Check if we're inside kitty
    if [ -z "${KITTY_WINDOW_ID:-}" ]; then
        printf "  ${YELLOW}Not inside kitty, opening new window...${NC}\n"

        # Create a session file
        local session_file="$DATA_DIR/kitty-session.conf"
        cat > "$session_file" << SESSION
# Aspen Cluster Session
new_tab Cluster
cd $PROJECT_DIR
title Node Logs
layout tall

# Node log windows
SESSION

        for id in $(seq 1 "$NODE_COUNT"); do
            echo "launch --title 'Node $id' tail -f $DATA_DIR/node$id/node.log" >> "$session_file"
        done

        # CLI window
        cat >> "$session_file" << SESSION

# CLI window
launch --title CLI bash -c 'export ASPEN_TICKET="$ticket"; printf "\\n\\033[0;36mAspen CLI\\033[0m\\n\\nTicket: \$ASPEN_TICKET\\n\\nCommands:\\n  $cli_bin --ticket \\\$ASPEN_TICKET cluster status\\n  $cli_bin --ticket \\\$ASPEN_TICKET kv set key value\\n\\n"; exec bash'
SESSION

        # Launch kitty with session
        kitty --session "$session_file" --title "Aspen Cluster" &
        printf "  Opened kitty window with cluster layout\n"
    else
        # Check if remote control is enabled
        if ! kitty @ ls >/dev/null 2>&1; then
            printf "  ${YELLOW}Kitty remote control is disabled.${NC}\n"
            printf "  To enable, add to ~/.config/kitty/kitty.conf:\n"
            printf "    ${CYAN}allow_remote_control yes${NC}\n"
            printf "    ${CYAN}listen_on unix:/tmp/kitty${NC}\n"
            printf "\n"
            printf "  ${BLUE}Manual log viewing:${NC}\n"
            for id in $(seq 1 "$NODE_COUNT"); do
                printf "    tail -f %s/node%d/node.log\n" "$DATA_DIR" "$id"
            done
            return 0
        fi

        printf "  ${CYAN}Already in kitty, creating new tab...${NC}\n"

        # Create new tab
        kitty @ launch --type=tab --tab-title "Node Logs" --cwd "$PROJECT_DIR" \
            tail -f "$DATA_DIR/node1/node.log"

        # Add windows for other nodes
        for id in $(seq 2 "$NODE_COUNT"); do
            kitty @ launch --type=window --title "Node $id" \
                tail -f "$DATA_DIR/node$id/node.log"
        done

        # Add CLI window
        kitty @ launch --type=window --title "CLI" --cwd "$PROJECT_DIR" \
            bash -c "export ASPEN_TICKET='$ticket'; printf '\\n\\033[0;36mAspen CLI\\033[0m\\n\\nTicket: \$ASPEN_TICKET\\n\\n'; exec bash"

        printf "  Created kitty layout with %d node logs + CLI\n" "$NODE_COUNT"
    fi
}

# Print final info
print_info() {
    local ticket="$1"
    local cli_bin="$2"

    printf "\n${BLUE}══════════════════════════════════════${NC}\n"
    printf "${GREEN}Aspen Cluster Ready${NC} (%d nodes)\n" "$NODE_COUNT"
    printf "${BLUE}══════════════════════════════════════${NC}\n\n"

    printf "Data dir:   %s\n" "$DATA_DIR"
    printf "Cookie:     %s\n" "$COOKIE"
    printf "Build:      %s\n" "$BUILD_MODE"
    printf "Workers:    %s\n" "$WORKERS_ENABLED"
    printf "CI:         %s\n" "$CI_ENABLED"
    printf "Blobs:      %s\n" "$BLOBS_ENABLED"
    printf "Docs:       %s\n" "$DOCS_ENABLED"
    printf "\nTicket:     %s/ticket.txt\n" "$DATA_DIR"

    printf "\n${CYAN}Quick commands:${NC}\n"
    printf "  export TICKET=\$(cat %s/ticket.txt)\n" "$DATA_DIR"
    printf "  %s --ticket \$TICKET cluster status\n" "$cli_bin"
    printf "  %s --ticket \$TICKET kv set foo bar\n" "$cli_bin"

    if [ "$CI_ENABLED" = true ]; then
        printf "\n${CYAN}CI commands:${NC}\n"
        printf "  %s --ticket \$TICKET ci list\n" "$cli_bin"
        printf "  %s --ticket \$TICKET ci run <repo_id>\n" "$cli_bin"
    fi

    printf "\n${BLUE}══════════════════════════════════════${NC}\n"
}

# Cleanup function
cleanup() {
    printf "\n${YELLOW}Shutting down cluster...${NC}\n"

    if [ -f "$DATA_DIR/pids" ]; then
        while read -r pid; do
            if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
                printf "  Stopping PID %s..." "$pid"
                kill "$pid" 2>/dev/null || true
                sleep 0.5
                if kill -0 "$pid" 2>/dev/null; then
                    kill -9 "$pid" 2>/dev/null || true
                fi
                printf " ${GREEN}done${NC}\n"
            fi
        done < "$DATA_DIR/pids"
    fi

    printf "${GREEN}Cluster stopped${NC}\n"
}

# Main
main() {
    trap cleanup EXIT INT TERM

    printf "${CYAN}"
    printf "    _                         \n"
    printf "   / \\   ___ _ __   ___ _ __  \n"
    printf "  / _ \\ / __| '_ \\ / _ \\ '_ \\ \n"
    printf " / ___ \\\\__ \\ |_) |  __/ | | |\n"
    printf "/_/   \\_\\___/ .__/ \\___|_| |_|\n"
    printf "            |_|               \n"
    printf "${NC}\n"
    printf "${BLUE}Kitty Cluster Launcher${NC}\n\n"

    # Build if needed
    if [ "$SKIP_BUILD" = false ]; then
        build_project
    fi

    # Get binaries
    local node_bin cli_bin
    node_bin=$(get_binary "aspen-node")
    cli_bin=$(get_binary "aspen-cli")

    if [ ! -x "$node_bin" ]; then
        printf "${RED}Error: aspen-node not found at %s${NC}\n" "$node_bin"
        exit 1
    fi

    if [ ! -x "$cli_bin" ]; then
        printf "${RED}Error: aspen-cli not found at %s${NC}\n" "$cli_bin"
        exit 1
    fi

    printf "Using binaries:\n"
    printf "  aspen-node: %s\n" "$node_bin"
    printf "  aspen-cli:  %s\n\n" "$cli_bin"

    # Setup data directory
    rm -rf "$DATA_DIR"
    mkdir -p "$DATA_DIR"
    : > "$DATA_DIR/pids"

    # Start nodes
    printf "${BLUE}Starting %d nodes...${NC}\n" "$NODE_COUNT"

    for id in $(seq 1 "$NODE_COUNT"); do
        start_node "$id" "$node_bin"
    done

    printf "\n${BLUE}Forming cluster...${NC}\n"

    # Wait for ticket
    local ticket
    if ! ticket=$(wait_for_ticket); then
        printf "${RED}Failed to get cluster ticket${NC}\n"
        exit 1
    fi

    # Initialize cluster
    if ! init_cluster "$cli_bin" "$ticket"; then
        printf "${YELLOW}Cluster init had issues but may still work${NC}\n"
    fi

    # Print info
    print_info "$ticket" "$cli_bin"

    # Open kitty layout
    open_kitty_layout "$ticket" "$cli_bin"

    # Keep running and monitor
    printf "\n${YELLOW}Cluster running. Press Ctrl+C to stop.${NC}\n\n"

    while true; do
        local all_running=true
        while read -r pid; do
            if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
                all_running=false
                break
            fi
        done < "$DATA_DIR/pids"

        if [ "$all_running" = false ]; then
            printf "${RED}A node died unexpectedly${NC}\n"
            exit 1
        fi

        sleep 5
    done
}

main "$@"
