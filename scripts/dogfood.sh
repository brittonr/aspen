#!/usr/bin/env bash
# Aspen Self-Hosting (Dogfooding) Script
#
# This script sets up and runs Aspen to host itself:
#   - Starts a 3-node cluster with Forge + CI + Nix Cache enabled
#   - Creates the Aspen repository in Forge
#   - Outputs instructions for pushing Aspen source and triggering CI
#
# Usage:
#   ./scripts/dogfood.sh [start|stop|status|init-repo|help]
#
# Environment variables:
#   ASPEN_DOGFOOD_DIR    - Data directory (default: /tmp/aspen-dogfood)
#   ASPEN_BUILD_RELEASE  - Build in release mode: true/false (default: false)
#   ASPEN_NODE_COUNT     - Number of nodes (default: 3)
#   ASPEN_FOREGROUND     - Run in foreground (default: true)
#
# Examples:
#   ./scripts/dogfood.sh start        # Start dogfood cluster
#   ./scripts/dogfood.sh init-repo    # Create Aspen repo in Forge
#   ./scripts/dogfood.sh stop         # Stop the cluster

set -eu

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Source shared functions
. "$SCRIPT_DIR/lib/cluster-common.sh"

# Configuration
DOGFOOD_DIR="${ASPEN_DOGFOOD_DIR:-/tmp/aspen-dogfood}"
BUILD_RELEASE="${ASPEN_BUILD_RELEASE:-false}"
NODE_COUNT="${ASPEN_NODE_COUNT:-3}"
FOREGROUND="${ASPEN_FOREGROUND:-true}"

# Fixed dogfood cluster settings
COOKIE="aspen-dogfood-cluster"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# PID file location
PID_FILE="$DOGFOOD_DIR/pids"

# Build features needed for dogfooding
DOGFOOD_FEATURES="ci,forge,nix-cache-gateway,shell-worker,blob"

print_header() {
    printf "${BLUE}============================================${NC}\n"
    printf "${GREEN}Aspen Self-Hosting (Dogfooding)${NC}\n"
    printf "${BLUE}============================================${NC}\n\n"
}

# Build binaries with required features
build_binaries() {
    printf "${BLUE}Building Aspen with dogfooding features...${NC}\n"

    local build_mode=""
    local target_dir="debug"
    if [ "$BUILD_RELEASE" = "true" ]; then
        build_mode="--release"
        target_dir="release"
    fi

    # Build with all required features
    if ! cargo build $build_mode --features "$DOGFOOD_FEATURES" --bin aspen-node --bin aspen-cli; then
        printf "${RED}Build failed${NC}\n"
        exit 1
    fi

    # Set binary paths
    ASPEN_NODE_BIN="$PROJECT_DIR/target/$target_dir/aspen-node"
    ASPEN_CLI_BIN="$PROJECT_DIR/target/$target_dir/aspen-cli"
    export ASPEN_NODE_BIN ASPEN_CLI_BIN

    printf "  aspen-node: %s\n" "$ASPEN_NODE_BIN"
    printf "  aspen-cli:  %s\n" "$ASPEN_CLI_BIN"
    printf "${GREEN}Build complete${NC}\n\n"
}

# Start a single node with dogfood settings
start_node() {
    local node_id="$1"
    local node_data_dir="$DOGFOOD_DIR/node$node_id"
    local log_file="$node_data_dir/node.log"

    mkdir -p "$node_data_dir"

    # Generate deterministic secret key
    local secret_key
    secret_key=$(printf '%064x' "$((1000 + node_id))")

    # Start node with all dogfooding features enabled
    RUST_LOG="${ASPEN_LOG_LEVEL:-info}" \
    ASPEN_BLOBS_ENABLED=true \
    ASPEN_DOCS_ENABLED=false \
    ASPEN_WORKERS_ENABLED=true \
    ASPEN_WORKER_COUNT=2 \
    ASPEN_CI_ENABLED=true \
    ASPEN_CI_AUTO_TRIGGER=true \
    ASPEN_FORGE_ENABLE_GOSSIP=true \
    ASPEN_NIX_CACHE_ENABLED=true \
    ASPEN_NIX_CACHE_PRIORITY=20 \
    ASPEN_IROH_ENABLE_MDNS=true \
    ASPEN_IROH_ENABLE_GOSSIP=true \
    "$ASPEN_NODE_BIN" \
        --node-id "$node_id" \
        --cookie "$COOKIE" \
        --data-dir "$node_data_dir" \
        --storage-backend redb \
        --iroh-secret-key "$secret_key" \
        > "$log_file" 2>&1 &

    local pid=$!
    printf '%s\n' "$pid" >> "$PID_FILE"
    printf "  Node %d: PID %d (log: %s)\n" "$node_id" "$pid" "$log_file"
}

# Wait for ticket from node 1 log
wait_for_ticket() {
    local log_file="$DOGFOOD_DIR/node1/node.log"
    local timeout=30
    local elapsed=0

    printf "  Waiting for node 1 to start"

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$log_file" ]; then
            local ticket
            ticket=$(grep -oE 'aspen[a-z2-7]{50,200}' "$log_file" 2>/dev/null | head -1 || true)

            if [ -n "$ticket" ]; then
                printf " ${GREEN}done${NC}\n"
                printf '%s' "$ticket" > "$DOGFOOD_DIR/ticket.txt"
                printf '%s' "$ticket"
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

# Get endpoint ID from a node's log file
get_endpoint_id() {
    local log_file="$1"
    # Strip ANSI escape sequences, then extract endpoint_id
    sed 's/\x1b\[[0-9;]*m//g' "$log_file" 2>/dev/null | \
        grep -oE 'endpoint_id=[a-f0-9]{64}' | head -1 | cut -d= -f2 || true
}

# Initialize the cluster
init_cluster() {
    local ticket="$1"

    printf "  Waiting for gossip discovery..."
    sleep 5
    printf " ${GREEN}done${NC}\n"

    # Initialize node 1
    printf "  Initializing cluster on node 1..."
    local attempts=0
    local max_attempts=5

    while [ "$attempts" -lt "$max_attempts" ]; do
        if "$ASPEN_CLI_BIN" --ticket "$ticket" cluster init >/dev/null 2>&1; then
            printf " ${GREEN}done${NC}\n"
            break
        fi
        attempts=$((attempts + 1))
        if [ "$attempts" -lt "$max_attempts" ]; then
            printf "."
            sleep 2
        fi
    done

    if [ "$attempts" -eq "$max_attempts" ]; then
        printf " ${RED}failed${NC}\n"
        return 1
    fi

    sleep 2

    # Add other nodes as learners
    if [ "$NODE_COUNT" -gt 1 ]; then
        printf "  Adding nodes 2-%d as learners...\n" "$NODE_COUNT"

        local id=2
        while [ "$id" -le "$NODE_COUNT" ]; do
            local endpoint_id
            endpoint_id=$(get_endpoint_id "$DOGFOOD_DIR/node$id/node.log")

            if [ -n "$endpoint_id" ]; then
                printf "    Node %d (%s...): " "$id" "$(echo "$endpoint_id" | cut -c1-16)"
                if "$ASPEN_CLI_BIN" --ticket "$ticket" cluster add-learner \
                    --node-id "$id" --addr "$endpoint_id" >/dev/null 2>&1; then
                    printf "${GREEN}added${NC}\n"
                else
                    printf "${YELLOW}skipped${NC}\n"
                fi
            else
                printf "    Node %d: ${RED}no endpoint ID${NC}\n" "$id"
            fi
            sleep 1
            id=$((id + 1))
        done

        # Promote all to voters
        printf "  Promoting all nodes to voters..."

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
        if "$ASPEN_CLI_BIN" --ticket "$ticket" cluster change-membership $members >/dev/null 2>&1; then
            printf " ${GREEN}done${NC}\n"
        else
            printf " ${YELLOW}skipped (may already be voters)${NC}\n"
        fi
    fi

    return 0
}

# Print cluster info with dogfood-specific details
print_info() {
    local ticket="$1"

    printf "\n${BLUE}======================================${NC}\n"
    printf "${GREEN}Aspen Dogfood Cluster Ready${NC} (%d nodes)\n" "$NODE_COUNT"
    printf "${BLUE}======================================${NC}\n"
    printf "\n"
    printf "Cookie:     %s\n" "$COOKIE"
    printf "Storage:    redb (persistent)\n"
    printf "Blobs:      enabled\n"
    printf "CI:         enabled (auto_trigger: true)\n"
    printf "Forge:      enabled (gossip: true)\n"
    printf "Nix Cache:  enabled (priority: 20)\n"
    printf "Workers:    enabled (2 per node)\n"
    printf "Data dir:   %s\n" "$DOGFOOD_DIR"
    printf "Ticket:     %s/ticket.txt\n" "$DOGFOOD_DIR"
    printf "\n"

    printf "${BLUE}Next Steps:${NC}\n"
    printf "  1. Initialize the Aspen repository:\n"
    printf "     %s init-repo\n" "$0"
    printf "\n"
    printf "  2. Add the Aspen remote to your git repo:\n"
    printf "     git remote add aspen aspen://%s/<repo_id>\n" "$ticket"
    printf "\n"
    printf "  3. Push to trigger CI:\n"
    printf "     git push aspen main\n"
    printf "\n"

    printf "${BLUE}CLI Commands:${NC}\n"
    printf "  %s --ticket %s cluster status\n" "$ASPEN_CLI_BIN" "$ticket"
    printf "  %s --ticket %s git list\n" "$ASPEN_CLI_BIN" "$ticket"
    printf "  %s --ticket %s ci status\n" "$ASPEN_CLI_BIN" "$ticket"
    printf "  %s --ticket %s blob list\n" "$ASPEN_CLI_BIN" "$ticket"
    printf "\n"

    printf "${BLUE}Stop cluster:${NC}\n"
    printf "  %s stop\n" "$0"
    printf "${BLUE}======================================${NC}\n"
}

# Initialize the Aspen repository in Forge
cmd_init_repo() {
    if [ ! -f "$DOGFOOD_DIR/ticket.txt" ]; then
        printf "${RED}Error: No cluster running. Start with: %s start${NC}\n" "$0"
        exit 1
    fi

    # Find CLI binary
    ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"
    if [ -z "$ASPEN_CLI_BIN" ] || [ ! -x "$ASPEN_CLI_BIN" ]; then
        printf "${RED}Error: aspen-cli binary not found${NC}\n"
        printf "Build with: cargo build --bin aspen-cli --features %s\n" "$DOGFOOD_FEATURES"
        exit 1
    fi

    local ticket
    ticket=$(cat "$DOGFOOD_DIR/ticket.txt")

    printf "${BLUE}Creating Aspen repository in Forge...${NC}\n"

    # Create the repository
    local output
    if ! output=$("$ASPEN_CLI_BIN" --ticket "$ticket" git init \
        --description "Aspen distributed systems platform (self-hosted)" \
        "aspen" 2>&1); then
        printf "${RED}Failed to create repository: %s${NC}\n" "$output"
        exit 1
    fi

    # Extract repo ID from output
    local repo_id
    repo_id=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || true)

    if [ -z "$repo_id" ]; then
        printf "${YELLOW}Repository may already exist. Output: %s${NC}\n" "$output"
        # Try to list repositories to get the ID
        local list_output
        list_output=$("$ASPEN_CLI_BIN" --ticket "$ticket" git list 2>&1 || true)
        printf "Existing repositories:\n%s\n" "$list_output"
        exit 1
    fi

    # Save repo ID
    printf '%s' "$repo_id" > "$DOGFOOD_DIR/repo_id.txt"

    printf "${GREEN}Repository created successfully!${NC}\n"
    printf "\n"
    printf "Repository ID: %s\n" "$repo_id"
    printf "\n"
    printf "${BLUE}To push Aspen source:${NC}\n"
    printf "  git remote add aspen aspen://%s/%s\n" "$ticket" "$repo_id"
    printf "  git push aspen main\n"
    printf "\n"
    printf "${BLUE}To enable CI watching for this repo:${NC}\n"
    printf "  Restart cluster with: ASPEN_CI_WATCHED_REPOS=%s ./scripts/dogfood.sh start\n" "$repo_id"
    printf "\n"
    printf "Or add to your config:\n"
    printf "  ci.watched_repos = [\"%s\"]\n" "$repo_id"
}

# Start the cluster
cmd_start() {
    print_header
    printf "Starting %d-node dogfood cluster\n\n" "$NODE_COUNT"

    # Build if binaries don't exist
    ASPEN_NODE_BIN="${ASPEN_NODE_BIN:-$(find_binary aspen-node)}"
    ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"

    if [ -z "$ASPEN_NODE_BIN" ] || [ -z "$ASPEN_CLI_BIN" ]; then
        build_binaries
    else
        printf "Using existing binaries:\n"
        printf "  aspen-node: %s\n" "$ASPEN_NODE_BIN"
        printf "  aspen-cli:  %s\n" "$ASPEN_CLI_BIN"
        printf "\n"
    fi

    # Clean up existing data
    if [ -d "$DOGFOOD_DIR" ]; then
        printf "${YELLOW}Cleaning previous dogfood data${NC}\n"
        cmd_stop 2>/dev/null || true
        rm -rf "$DOGFOOD_DIR"
    fi

    mkdir -p "$DOGFOOD_DIR"
    : > "$PID_FILE"

    # Start nodes
    printf "${BLUE}Starting nodes...${NC}\n"
    local id=1
    while [ "$id" -le "$NODE_COUNT" ]; do
        start_node "$id"
        id=$((id + 1))
    done

    printf "\n${BLUE}Forming cluster...${NC}\n"

    local ticket
    if ! ticket=$(wait_for_ticket); then
        printf "${RED}Failed to get cluster ticket${NC}\n" >&2
        cmd_stop
        exit 1
    fi

    if ! init_cluster "$ticket"; then
        printf "${YELLOW}Cluster init had issues, but may still work${NC}\n"
    fi

    print_info "$ticket"

    if [ "$FOREGROUND" = "true" ]; then
        printf "\n${YELLOW}Running in foreground. Press Ctrl+C to stop.${NC}\n"
        trap cmd_stop EXIT INT TERM QUIT

        # Monitor nodes
        while true; do
            local all_running=true
            while read -r pid; do
                if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
                    all_running=false
                    break
                fi
            done < "$PID_FILE"

            if [ "$all_running" = "false" ]; then
                printf "${RED}A node died unexpectedly${NC}\n"
                exit 1
            fi

            sleep 5
        done
    fi
}

# Stop the cluster
cmd_stop() {
    if [ ! -f "$PID_FILE" ]; then
        printf "${YELLOW}No cluster running (no PID file at %s)${NC}\n" "$PID_FILE"
        return 0
    fi

    printf "${BLUE}Stopping dogfood cluster...${NC}\n"

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
    done < "$PID_FILE"

    rm -f "$PID_FILE"
    printf "${GREEN}Cluster stopped${NC}\n"
}

# Show cluster status
cmd_status() {
    if [ ! -f "$PID_FILE" ]; then
        printf "${YELLOW}No cluster running${NC}\n"
        return 1
    fi

    printf "${BLUE}Dogfood Cluster Status${NC}\n\n"

    local running=0
    local id=1
    while read -r pid; do
        if [ -n "$pid" ]; then
            if kill -0 "$pid" 2>/dev/null; then
                printf "  Node %d: ${GREEN}running${NC} (PID %s)\n" "$id" "$pid"
                running=$((running + 1))
            else
                printf "  Node %d: ${RED}stopped${NC} (was PID %s)\n" "$id" "$pid"
            fi
            id=$((id + 1))
        fi
    done < "$PID_FILE"

    printf "\n"

    if [ -f "$DOGFOOD_DIR/ticket.txt" ]; then
        local ticket
        ticket=$(cat "$DOGFOOD_DIR/ticket.txt")
        printf "Ticket: %s\n\n" "$ticket"

        ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"
        if [ -n "$ASPEN_CLI_BIN" ]; then
            printf "Cluster info:\n"
            "$ASPEN_CLI_BIN" --ticket "$ticket" cluster status 2>/dev/null || printf "  (unable to get status)\n"
        fi
    fi

    if [ -f "$DOGFOOD_DIR/repo_id.txt" ]; then
        printf "\nAspen Repo ID: %s\n" "$(cat "$DOGFOOD_DIR/repo_id.txt")"
    fi

    if [ "$running" -eq 0 ]; then
        return 1
    fi
    return 0
}

# Show usage
cmd_help() {
    printf "Aspen Self-Hosting (Dogfooding) Script\n"
    printf "\n"
    printf "Usage: %s {start|stop|status|init-repo|help}\n" "$0"
    printf "\n"
    printf "Commands:\n"
    printf "  start      - Build and start the dogfood cluster\n"
    printf "  stop       - Stop all nodes\n"
    printf "  status     - Show cluster status\n"
    printf "  init-repo  - Create the Aspen repository in Forge\n"
    printf "  help       - Show this help message\n"
    printf "\n"
    printf "Environment variables:\n"
    printf "  ASPEN_DOGFOOD_DIR    - Data directory (default: /tmp/aspen-dogfood)\n"
    printf "  ASPEN_BUILD_RELEASE  - Build release: true/false (default: false)\n"
    printf "  ASPEN_NODE_COUNT     - Number of nodes (default: 3)\n"
    printf "  ASPEN_LOG_LEVEL      - Log level (default: info)\n"
    printf "  ASPEN_FOREGROUND     - Run in foreground (default: true)\n"
    printf "  ASPEN_CI_WATCHED_REPOS - Comma-separated repo IDs to watch\n"
    printf "\n"
    printf "Features enabled:\n"
    printf "  - Forge (Git hosting with gossip)\n"
    printf "  - CI (auto-trigger on ref updates)\n"
    printf "  - Nix Cache (HTTP/3 binary cache)\n"
    printf "  - Blob Storage (with replication)\n"
    printf "  - Workers (for CI job execution)\n"
    printf "\n"
    printf "Workflow:\n"
    printf "  1. %s start       # Start the cluster\n" "$0"
    printf "  2. %s init-repo   # Create Aspen repo\n" "$0"
    printf "  3. git remote add aspen aspen://<ticket>/<repo_id>\n"
    printf "  4. git push aspen main   # Triggers CI\n"
    printf "\n"
}

# Main
main() {
    local cmd="${1:-help}"
    shift || true

    case "$cmd" in
        start)
            cmd_start
            ;;
        stop)
            cmd_stop
            ;;
        status)
            cmd_status
            ;;
        init-repo)
            cmd_init_repo
            ;;
        help|--help|-h)
            cmd_help
            ;;
        *)
            printf "${RED}Unknown command: %s${NC}\n" "$cmd"
            cmd_help
            exit 1
            ;;
    esac
}

main "$@"
