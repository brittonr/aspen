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
CI_DEMO=false

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
        --ci-demo)
            CI_DEMO=true
            CI_ENABLED=true
            WORKERS_ENABLED=true
            BLOBS_ENABLED=true
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
            printf "  --ci-demo       Full CI demo (enables ci, workers, blobs)\n"
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

    # Use stderr for status messages so they don't pollute the ticket output
    printf "  Waiting for node 1 to start" >&2

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$log_file" ]; then
            local ticket
            ticket=$(grep -oE 'aspen[a-z2-7]{50,200}' "$log_file" 2>/dev/null | head -1 || true)

            if [ -n "$ticket" ]; then
                printf " ${GREEN}done${NC}\n" >&2
                echo "$ticket" > "$DATA_DIR/ticket.txt"
                echo "$ticket"
                return 0
            fi
        fi

        printf "." >&2
        sleep 1
        elapsed=$((elapsed + 1))
    done

    printf " ${RED}timeout${NC}\n" >&2
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

# Run CI demo - creates a test repo with CI config and triggers a pipeline
run_ci_demo() {
    local cli_bin="$1"
    local ticket="$2"

    printf "\n${BLUE}══════════════════════════════════════${NC}\n"
    printf "${CYAN}Running CI Demo${NC}\n"
    printf "${BLUE}══════════════════════════════════════${NC}\n\n"

    # Step 1: Create test repository
    printf "  ${CYAN}Step 1: Creating test repository...${NC}\n"
    local repo_output
    if ! repo_output=$("$cli_bin" --ticket "$ticket" --json git init ci-demo-repo --description "CI Demo Repository" 2>&1); then
        printf "    ${YELLOW}Warning: Could not create repo (may already exist)${NC}\n"
        printf "    Output: %s\n" "$repo_output"
        # Try to get existing repo
        repo_output=$("$cli_bin" --ticket "$ticket" --json git list 2>&1 || true)
    fi

    # Extract repo_id from output (handle both new and existing)
    local repo_id
    repo_id=$(echo "$repo_output" | grep -oE '"repo_id"\s*:\s*"[a-f0-9]{64}"' | head -1 | cut -d'"' -f4 || true)
    if [ -z "$repo_id" ]; then
        repo_id=$(echo "$repo_output" | grep -oE '[a-f0-9]{64}' | head -1 || true)
    fi

    if [ -z "$repo_id" ]; then
        printf "    ${RED}Could not get repo_id${NC}\n"
        return 1
    fi

    printf "    Repository ID: ${GREEN}%s${NC}\n" "${repo_id:0:16}..."

    # Step 2: Create CI config content
    printf "  ${CYAN}Step 2: Creating CI configuration...${NC}\n"

    local ci_config
    ci_config=$(cat << 'CICONFIG'
{
  name = "ci-demo",
  version = "1",
  stages = [
    {
      name = "test",
      jobs = [
        {
          name = "echo-test",
          job_type = "shell",
          command = "echo 'Hello from CI!'",
          timeout_secs = 60,
        },
      ],
    },
  ],
}
CICONFIG
)

    # Store CI config as blob
    local config_file="$DATA_DIR/ci.ncl"
    echo "$ci_config" > "$config_file"
    local blob_output
    blob_output=$("$cli_bin" --ticket "$ticket" --json git store-blob --repo "$repo_id" "$config_file" 2>&1 || true)
    local config_hash
    config_hash=$(echo "$blob_output" | grep -oE '[a-f0-9]{64}' | head -1 || true)

    if [ -z "$config_hash" ]; then
        printf "    ${RED}Failed to store CI config as blob${NC}\n"
        printf "    Output: %s\n" "$blob_output"
        return 1
    fi

    printf "    CI config blob: ${GREEN}%s${NC}\n" "${config_hash:0:16}..."

    # Step 3: Create a README blob
    printf "  ${CYAN}Step 3: Creating README...${NC}\n"
    local readme_file="$DATA_DIR/README.md"
    echo "# CI Demo Repository" > "$readme_file"
    echo "" >> "$readme_file"
    echo "This repository demonstrates the Aspen CI system." >> "$readme_file"

    local readme_output
    readme_output=$("$cli_bin" --ticket "$ticket" --json git store-blob --repo "$repo_id" "$readme_file" 2>&1 || true)
    local readme_hash
    readme_hash=$(echo "$readme_output" | grep -oE '[a-f0-9]{64}' | head -1 || true)

    if [ -z "$readme_hash" ]; then
        printf "    ${YELLOW}Could not store README (continuing)${NC}\n"
    else
        printf "    README blob: ${GREEN}%s${NC}\n" "${readme_hash:0:16}..."
    fi

    # Step 4: Create .aspen directory tree
    printf "  ${CYAN}Step 4: Creating directory structure...${NC}\n"

    # Create .aspen tree with ci.ncl
    local aspen_tree_output
    aspen_tree_output=$("$cli_bin" --ticket "$ticket" --json git create-tree --repo "$repo_id" \
        --entry "100644:ci.ncl:$config_hash" 2>&1 || true)
    local aspen_tree_hash
    aspen_tree_hash=$(echo "$aspen_tree_output" | grep -oE '[a-f0-9]{64}' | head -1 || true)

    if [ -z "$aspen_tree_hash" ]; then
        printf "    ${RED}Failed to create .aspen tree${NC}\n"
        return 1
    fi

    printf "    .aspen tree: ${GREEN}%s${NC}\n" "${aspen_tree_hash:0:16}..."

    # Step 5: Create root tree
    printf "  ${CYAN}Step 5: Creating root tree...${NC}\n"

    local tree_entries="--entry 040000:.aspen:$aspen_tree_hash"
    if [ -n "$readme_hash" ]; then
        tree_entries="$tree_entries --entry 100644:README.md:$readme_hash"
    fi

    local root_tree_output
    root_tree_output=$("$cli_bin" --ticket "$ticket" --json git create-tree --repo "$repo_id" \
        $tree_entries 2>&1 || true)
    local root_tree_hash
    root_tree_hash=$(echo "$root_tree_output" | grep -oE '[a-f0-9]{64}' | head -1 || true)

    if [ -z "$root_tree_hash" ]; then
        printf "    ${RED}Failed to create root tree${NC}\n"
        return 1
    fi

    printf "    Root tree: ${GREEN}%s${NC}\n" "${root_tree_hash:0:16}..."

    # Step 6: Create commit
    printf "  ${CYAN}Step 6: Creating commit...${NC}\n"

    local commit_output
    commit_output=$("$cli_bin" --ticket "$ticket" --json git commit --repo "$repo_id" \
        --tree "$root_tree_hash" \
        --message "Initial commit with CI config" 2>&1 || true)
    local commit_hash
    commit_hash=$(echo "$commit_output" | grep -oE '[a-f0-9]{64}' | head -1 || true)

    if [ -z "$commit_hash" ]; then
        printf "    ${RED}Failed to create commit${NC}\n"
        printf "    Output: %s\n" "$commit_output"
        return 1
    fi

    printf "    Commit: ${GREEN}%s${NC}\n" "${commit_hash:0:16}..."

    # Step 7: Push to main branch
    printf "  ${CYAN}Step 7: Pushing to main branch...${NC}\n"

    local push_output
    push_output=$("$cli_bin" --ticket "$ticket" --json git push --repo "$repo_id" \
        --ref-name "heads/main" --hash "$commit_hash" 2>&1 || true)

    printf "    Pushed to refs/heads/main: %s\n" "${push_output}"

    # Step 8: Trigger CI pipeline
    printf "  ${CYAN}Step 8: Triggering CI pipeline...${NC}\n"

    local ci_output
    ci_output=$("$cli_bin" --ticket "$ticket" --json ci run "$repo_id" --ref-name "refs/heads/main" 2>&1 || true)
    local run_id
    run_id=$(echo "$ci_output" | grep -oE '"run_id"\s*:\s*"[a-f0-9-]{36}"' | head -1 | cut -d'"' -f4 || true)

    if [ -z "$run_id" ]; then
        printf "    ${YELLOW}Note: CI trigger returned: %s${NC}\n" "$ci_output"
        printf "    (This may indicate workers need more time to start)\n"
    else
        printf "    Pipeline run ID: ${GREEN}%s${NC}\n" "$run_id"

        # Step 9: Check status
        printf "  ${CYAN}Step 9: Checking pipeline status...${NC}\n"
        sleep 2  # Give it a moment

        local status_output
        status_output=$("$cli_bin" --ticket "$ticket" --json ci status "$run_id" 2>&1 || true)
        local status
        status=$(echo "$status_output" | grep -oE '"status"\s*:\s*"[^"]+"' | head -1 | cut -d'"' -f4 || true)
        printf "    Status: ${GREEN}%s${NC}\n" "${status:-unknown}"
    fi

    printf "\n${BLUE}══════════════════════════════════════${NC}\n"
    printf "${GREEN}CI Demo Complete!${NC}\n"
    printf "${BLUE}══════════════════════════════════════${NC}\n\n"

    printf "To monitor the pipeline:\n"
    printf "  %s --ticket \$TICKET ci status %s\n" "$cli_bin" "${run_id:-<run_id>}"
    printf "  %s --ticket \$TICKET ci list --repo-id %s\n" "$cli_bin" "$repo_id"

    return 0
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
    printf "CI Demo:    %s\n" "$CI_DEMO"
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

    # Run CI demo if requested
    if [ "$CI_DEMO" = true ]; then
        printf "\n${CYAN}Waiting for workers to initialize...${NC}\n"
        sleep 5  # Give workers time to start

        if ! run_ci_demo "$cli_bin" "$ticket"; then
            printf "${YELLOW}CI demo encountered issues but cluster is still running${NC}\n"
        fi
    fi

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
