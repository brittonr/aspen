#!/usr/bin/env bash
# Simple Aspen cluster launcher
# Spawns N nodes in the background, forms a cluster, and provides a ticket for connection
#
# Usage: ./scripts/cluster.sh [start|stop|status|logs]
#        nix run .#cluster
#
# Environment variables:
#   ASPEN_NODE_COUNT  - Number of nodes to spawn (default: 3)
#   ASPEN_COOKIE      - Cluster authentication cookie (default: aspen-cluster-$$)
#   ASPEN_LOG_LEVEL   - Log level for nodes (default: info)
#   ASPEN_DATA_DIR    - Base data directory (default: /tmp/aspen-cluster-$$)
#   ASPEN_STORAGE     - Storage backend: inmemory, redb (default: inmemory)
#   ASPEN_BLOBS       - Enable blob storage: true/false (default: false)
#   ASPEN_DOCS        - Enable iroh-docs CRDT sync: true/false (default: false)
#   ASPEN_DHT         - Enable DHT content discovery: true/false (default: false)
#   ASPEN_DHT_PORT    - Base DHT port (incremented per node, default: 6881)
#   ASPEN_FOREGROUND  - Run in foreground (don't daemonize): true/false (default: true)

set -euo pipefail

# Configuration with defaults
NODE_COUNT="${ASPEN_NODE_COUNT:-3}"
COOKIE="${ASPEN_COOKIE:-aspen-cluster-$$}"
LOG_LEVEL="${ASPEN_LOG_LEVEL:-info}"
DATA_DIR="${ASPEN_DATA_DIR:-/tmp/aspen-cluster-$$}"
STORAGE="${ASPEN_STORAGE:-inmemory}"
BLOBS_ENABLED="${ASPEN_BLOBS:-false}"
DOCS_ENABLED="${ASPEN_DOCS:-false}"
DHT_ENABLED="${ASPEN_DHT:-false}"
DHT_BASE_PORT="${ASPEN_DHT_PORT:-6881}"
FOREGROUND="${ASPEN_FOREGROUND:-true}"
WORKERS_ENABLED="${ASPEN_WORKERS:-false}"
WORKER_COUNT="${ASPEN_WORKER_COUNT:-4}"

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# PID file location
PID_FILE="$DATA_DIR/pids"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Source shared functions
source "$SCRIPT_DIR/lib/cluster-common.sh"

ASPEN_NODE_BIN="${ASPEN_NODE_BIN:-$(find_binary aspen-node)}"
ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"

# Start a single node
start_node() {
    local node_id="$1"
    local node_data_dir="$DATA_DIR/node$node_id"
    local secret_key
    secret_key=$(generate_secret_key "$node_id")
    local log_file="$node_data_dir/node.log"

    mkdir -p "$node_data_dir"

    # Calculate DHT port for this node (each node gets unique port)
    local dht_port=$((DHT_BASE_PORT + node_id - 1))

    # Build worker args if enabled
    local worker_args=""
    if [ "$WORKERS_ENABLED" = "true" ]; then
        worker_args="--enable-workers --worker-count $WORKER_COUNT"
    fi

    RUST_LOG="$LOG_LEVEL" \
    ASPEN_BLOBS_ENABLED="$BLOBS_ENABLED" \
    ASPEN_DOCS_ENABLED="$DOCS_ENABLED" \
    ASPEN_CONTENT_DISCOVERY_ENABLED="$DHT_ENABLED" \
    ASPEN_CONTENT_DISCOVERY_SERVER_MODE="$DHT_ENABLED" \
    ASPEN_CONTENT_DISCOVERY_DHT_PORT="$dht_port" \
    ASPEN_CONTENT_DISCOVERY_AUTO_ANNOUNCE="$DHT_ENABLED" \
    "$ASPEN_NODE_BIN" \
        --node-id "$node_id" \
        --cookie "$COOKIE" \
        --data-dir "$node_data_dir" \
        --storage-backend "$STORAGE" \
        --iroh-secret-key "$secret_key" \
        $worker_args \
        > "$log_file" 2>&1 &

    local pid=$!
    echo "$pid" >> "$PID_FILE"
    printf "  Node %d: PID %d (log: %s)\n" "$node_id" "$pid" "$log_file"
}

# Wait for ticket from node 1 log
wait_for_ticket() {
    local log_file="$DATA_DIR/node1/node.log"
    local timeout=30
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
            endpoint_id=$(get_endpoint_id "$DATA_DIR/node$id/node.log")

            if [ -n "$endpoint_id" ]; then
                printf "    Node %d (%s...): " "$id" "${endpoint_id:0:16}"
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

# Print cluster info
print_info() {
    local ticket="$1"

    printf "\n${BLUE}======================================${NC}\n"
    printf "${GREEN}Aspen Cluster Ready${NC} (%d nodes)\n" "$NODE_COUNT"
    printf "${BLUE}======================================${NC}\n"
    printf "\n"
    printf "Cookie:   %s\n" "$COOKIE"
    printf "Storage:  %s\n" "$STORAGE"
    printf "Blobs:    %s\n" "$BLOBS_ENABLED"
    printf "Docs:     %s\n" "$DOCS_ENABLED"
    printf "DHT:      %s (ports: %d-%d)\n" "$DHT_ENABLED" "$DHT_BASE_PORT" "$((DHT_BASE_PORT + NODE_COUNT - 1))"
    printf "Workers:  %s (count: %s per node)\n" "$WORKERS_ENABLED" "$WORKER_COUNT"
    printf "Data dir: %s\n" "$DATA_DIR"
    printf "Ticket:   %s/ticket.txt\n" "$DATA_DIR"
    printf "\n"
    printf "${BLUE}Connect with CLI:${NC}\n"
    printf "  %s --ticket %s cluster status\n" "$ASPEN_CLI_BIN" "$ticket"
    printf "  %s --ticket %s kv set mykey 'hello'\n" "$ASPEN_CLI_BIN" "$ticket"
    printf "  %s --ticket %s kv get mykey\n" "$ASPEN_CLI_BIN" "$ticket"
    if [ "$BLOBS_ENABLED" = "true" ]; then
        printf "\n"
        printf "${BLUE}Blob storage:${NC}\n"
        printf "  %s --ticket %s blob add /path/to/file\n" "$ASPEN_CLI_BIN" "$ticket"
        printf "  %s --ticket %s blob list\n" "$ASPEN_CLI_BIN" "$ticket"
        if [ "$DHT_ENABLED" = "true" ]; then
            printf "  %s --ticket %s blob download-by-hash <hash>  # DHT lookup\n" "$ASPEN_CLI_BIN" "$ticket"
        fi
    fi
    printf "\n"
    printf "${BLUE}Connect with TUI:${NC}\n"
    printf "  aspen-tui --ticket %s\n" "$ticket"
    printf "\n"
    printf "${BLUE}Stop cluster:${NC}\n"
    printf "  %s stop\n" "$0"
    printf "${BLUE}======================================${NC}\n"
}

# Start the cluster
cmd_start() {
    printf "${BLUE}Aspen Cluster${NC} - Starting %d nodes\n\n" "$NODE_COUNT"

    check_prerequisites

    printf "Using aspen-node: %s\n" "$ASPEN_NODE_BIN"
    printf "Using aspen-cli:  %s\n\n" "$ASPEN_CLI_BIN"

    # Clean up existing data
    if [ -d "$DATA_DIR" ]; then
        printf "${YELLOW}Cleaning previous cluster data${NC}\n"
        cmd_stop 2>/dev/null || true
        rm -rf "$DATA_DIR"
    fi

    mkdir -p "$DATA_DIR"
    : > "$PID_FILE"

    # Start nodes
    printf "${BLUE}Starting nodes...${NC}\n"
    for id in $(seq 1 "$NODE_COUNT"); do
        start_node "$id"
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

    printf "${BLUE}Stopping cluster...${NC}\n"

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

    printf "${BLUE}Cluster Status${NC}\n\n"

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

    if [ -f "$DATA_DIR/ticket.txt" ]; then
        local ticket
        ticket=$(cat "$DATA_DIR/ticket.txt")
        printf "Ticket: %s\n\n" "$ticket"
        printf "Cluster info:\n"
        "$ASPEN_CLI_BIN" --ticket "$ticket" cluster status 2>/dev/null || printf "  (unable to get status)\n"
    fi

    if [ "$running" -eq 0 ]; then
        return 1
    fi
    return 0
}

# Show logs
cmd_logs() {
    local node_id="${1:-1}"
    local log_file="$DATA_DIR/node$node_id/node.log"

    if [ ! -f "$log_file" ]; then
        printf "${RED}No log file for node %s${NC}\n" "$node_id"
        printf "Available: %s/node*/node.log\n" "$DATA_DIR"
        exit 1
    fi

    tail -f "$log_file"
}

# Main
main() {
    local cmd="${1:-start}"
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
        logs)
            cmd_logs "${1:-1}"
            ;;
        restart)
            cmd_stop
            sleep 1
            cmd_start
            ;;
        *)
            printf "Usage: %s {start|stop|status|logs [node_id]|restart}\n" "$0"
            printf "\n"
            printf "Commands:\n"
            printf "  start   - Start the cluster (default)\n"
            printf "  stop    - Stop all nodes\n"
            printf "  status  - Show cluster status\n"
            printf "  logs N  - Tail logs for node N (default: 1)\n"
            printf "  restart - Stop then start\n"
            printf "\n"
            printf "Environment variables:\n"
            printf "  ASPEN_NODE_COUNT  - Number of nodes (default: 3)\n"
            printf "  ASPEN_COOKIE      - Cluster cookie (default: aspen-cluster-PID)\n"
            printf "  ASPEN_DATA_DIR    - Data directory (default: /tmp/aspen-cluster-PID)\n"
            printf "  ASPEN_STORAGE     - Storage: inmemory, redb (default: inmemory)\n"
            printf "  ASPEN_BLOBS       - Enable blobs: true/false (default: false)\n"
            printf "  ASPEN_DOCS        - Enable docs: true/false (default: false)\n"
            printf "  ASPEN_DHT         - Enable DHT content discovery: true/false (default: false)\n"
            printf "  ASPEN_DHT_PORT    - Base DHT port (default: 6881)\n"
            printf "  ASPEN_FOREGROUND  - Run in foreground (default: true)\n"
            printf "\n"
            printf "Examples:\n"
            printf "  %s start                            # Start 3-node cluster\n" "$0"
            printf "  ASPEN_NODE_COUNT=5 %s start         # Start 5-node cluster\n" "$0"
            printf "  ASPEN_DOCS=true %s start            # Start with docs sync enabled\n" "$0"
            printf "  ASPEN_BLOBS=true ASPEN_DHT=true %s  # Blobs with global DHT discovery\n" "$0"
            printf "  %s logs 2                           # Tail logs from node 2\n" "$0"
            exit 1
            ;;
    esac
}

main "$@"
