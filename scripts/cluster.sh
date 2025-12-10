#!/usr/bin/env bash
# Aspen 3-node cluster launcher
# Usage: nix run .#cluster
#
# Environment variables:
#   ASPEN_NODE_COUNT  - Number of nodes to spawn (default: 3)
#   ASPEN_BASE_HTTP   - Base HTTP port (default: 21001)
#   ASPEN_BASE_RACTOR - Base Ractor cluster port (default: 26001)
#   ASPEN_COOKIE      - Cluster authentication cookie (default: aspen-cluster)
#   ASPEN_LOG_LEVEL   - Log level for nodes (default: info)
#   ASPEN_DATA_DIR    - Base data directory (default: /tmp/aspen-cluster)
#   ASPEN_STORAGE     - Storage backend: inmemory, sqlite, redb (default: inmemory)
#   ASPEN_NO_INIT     - If set, don't auto-initialize the cluster
set -euo pipefail

# Configuration with defaults
NODE_COUNT="${ASPEN_NODE_COUNT:-3}"
BASE_HTTP="${ASPEN_BASE_HTTP:-21001}"
BASE_RACTOR="${ASPEN_BASE_RACTOR:-26001}"
COOKIE="${ASPEN_COOKIE:-aspen-cluster}"
LOG_LEVEL="${ASPEN_LOG_LEVEL:-info}"
DATA_DIR="${ASPEN_DATA_DIR:-/tmp/aspen-cluster}"
STORAGE="${ASPEN_STORAGE:-inmemory}"
NO_INIT="${ASPEN_NO_INIT:-}"

# Resolve script directory and binary path
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# Find aspen-node binary - check multiple locations
if [[ -n "${ASPEN_NODE_BIN:-}" ]] && [[ -x "$ASPEN_NODE_BIN" ]]; then
    BIN="$ASPEN_NODE_BIN"
elif [[ -x "$ROOT_DIR/result/bin/aspen-node" ]]; then
    BIN="$ROOT_DIR/result/bin/aspen-node"
elif command -v aspen-node >/dev/null 2>&1; then
    BIN="$(command -v aspen-node)"
else
    echo "Error: aspen-node binary not found" >&2
    echo "Set ASPEN_NODE_BIN or ensure aspen-node is in PATH" >&2
    exit 1
fi

echo "Using aspen-node: $BIN"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Array to track PIDs
declare -a PIDS=()

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Shutting down cluster...${NC}"
    local exit_code=0
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            echo "Stopping node (PID: $pid)"
            kill "$pid" 2>/dev/null || true
        fi
    done
    # Wait for graceful shutdown
    for _ in $(seq 1 30); do
        local running=0
        for pid in "${PIDS[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                running=1
                break
            fi
        done
        if (( running == 0 )); then
            break
        fi
        sleep 0.1
    done
    # Force kill if still running
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            echo "Force killing node (PID: $pid)"
            kill -9 "$pid" 2>/dev/null || true
        fi
    done
    echo -e "${GREEN}Cluster stopped${NC}"
    exit $exit_code
}

trap cleanup EXIT INT TERM

# Wait for HTTP port to be available
wait_for_port() {
    local port=$1
    local timeout=${2:-30}
    local start_time=$(date +%s)

    while ! nc -z 127.0.0.1 "$port" 2>/dev/null; do
        local now=$(date +%s)
        if (( now - start_time > timeout )); then
            echo -e "${RED}Timeout waiting for port $port${NC}" >&2
            return 1
        fi
        sleep 0.2
    done
    return 0
}

# Start a single node
start_node() {
    local id=$1
    local http_port=$((BASE_HTTP + id - 1))
    local ractor_port=$((BASE_RACTOR + id - 1))
    local log_file="$DATA_DIR/node$id.log"
    local node_data_dir="$DATA_DIR/node$id"

    # Generate deterministic secret key from node ID
    printf -v secret "%064x" "$((1000 + id))"

    # Build command
    local cmd=("$BIN"
        --node-id "$id"
        --http-addr "127.0.0.1:$http_port"
        --host "127.0.0.1"
        --port "$ractor_port"
        --cookie "$COOKIE"
        --iroh-secret-key "$secret"
        --control-backend "raft_actor"
    )

    # Always create data directory and pass it to node
    # Even inmemory storage needs a data dir for metadata.redb
    mkdir -p "$node_data_dir"
    cmd+=(--storage-backend "$STORAGE" --data-dir "$node_data_dir")

    # Start node in background
    RUST_LOG="$LOG_LEVEL" RUST_BACKTRACE=1 "${cmd[@]}" >"$log_file" 2>&1 &
    local pid=$!
    PIDS+=("$pid")

    # Wait for node to be ready
    if ! wait_for_port "$http_port" 30; then
        echo -e "${RED}Node $id failed to start. Check $log_file${NC}" >&2
        tail -20 "$log_file" >&2 || true
        return 1
    fi

    echo -e "${GREEN}Node $id${NC} started on HTTP :$http_port, Ractor :$ractor_port (PID: $pid)"
}

# Initialize the cluster
init_cluster() {
    local leader_port=$BASE_HTTP

    # Build member list with raft_addr included
    local members="["
    for ((i=1; i<=NODE_COUNT; i++)); do
        local http_port=$((BASE_HTTP + i - 1))
        local ractor_port=$((BASE_RACTOR + i - 1))
        if (( i > 1 )); then
            members+=","
        fi
        # Include raft_addr pointing to the Ractor cluster port
        members+="{\"id\":$i,\"addr\":\"127.0.0.1:$http_port\",\"raft_addr\":\"127.0.0.1:$ractor_port\"}"
    done
    members+="]"

    echo -e "${BLUE}Initializing cluster with $NODE_COUNT nodes...${NC}"

    local response
    response=$(curl -s -X POST "http://127.0.0.1:$leader_port/init" \
        -H "Content-Type: application/json" \
        -d "{\"initial_members\":$members}" 2>&1)

    if echo "$response" | grep -q '"members"'; then
        echo -e "${GREEN}Cluster initialized successfully${NC}"
        return 0
    else
        echo -e "${YELLOW}Cluster initialization response: $response${NC}"
        return 1
    fi
}

# Check cluster health
check_health() {
    echo -e "\n${BLUE}Cluster Status:${NC}"
    echo "----------------------------------------"
    for ((i=1; i<=NODE_COUNT; i++)); do
        local http_port=$((BASE_HTTP + i - 1))
        local health
        health=$(curl -s "http://127.0.0.1:$http_port/health" 2>/dev/null || echo '{"status":"unreachable"}')
        local status
        status=$(echo "$health" | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4)

        case "$status" in
            healthy)
                echo -e "  Node $i (:$http_port): ${GREEN}$status${NC}"
                ;;
            degraded)
                echo -e "  Node $i (:$http_port): ${YELLOW}$status${NC}"
                ;;
            *)
                echo -e "  Node $i (:$http_port): ${RED}$status${NC}"
                ;;
        esac
    done
    echo "----------------------------------------"
}

# Print cluster info
print_info() {
    echo -e "\n${BLUE}Aspen Cluster Information${NC}"
    echo "========================================"
    echo "Nodes:        $NODE_COUNT"
    echo "HTTP ports:   $BASE_HTTP - $((BASE_HTTP + NODE_COUNT - 1))"
    echo "Ractor ports: $BASE_RACTOR - $((BASE_RACTOR + NODE_COUNT - 1))"
    echo "Cookie:       $COOKIE"
    echo "Storage:      $STORAGE"
    echo "Log dir:      $DATA_DIR"
    echo ""
    echo "HTTP endpoints:"
    for ((i=1; i<=NODE_COUNT; i++)); do
        echo "  Node $i: http://127.0.0.1:$((BASE_HTTP + i - 1))"
    done
    echo ""
    echo "TUI connection:"
    local nodes_arg=""
    for ((i=1; i<=NODE_COUNT; i++)); do
        nodes_arg+=" --nodes http://127.0.0.1:$((BASE_HTTP + i - 1))"
    done
    echo "  nix run .#aspen-tui --$nodes_arg"
    echo ""
    echo "Useful commands:"
    echo "  Health: curl http://127.0.0.1:$BASE_HTTP/health | jq"
    echo "  Metrics: curl http://127.0.0.1:$BASE_HTTP/metrics"
    echo "  Write: curl -X POST http://127.0.0.1:$BASE_HTTP/write -H 'Content-Type: application/json' -d '{\"command\":{\"Set\":{\"key\":\"test\",\"value\":\"hello\"}}}'"
    echo "  Read: curl -X POST http://127.0.0.1:$BASE_HTTP/read -H 'Content-Type: application/json' -d '{\"key\":\"test\"}'"
    echo "========================================"
}

# Main execution
main() {
    echo -e "${BLUE}Starting Aspen $NODE_COUNT-node cluster${NC}"
    echo ""

    # Kill any existing aspen-node processes
    # This prevents port conflicts from previous runs
    if pgrep -f "aspen-node" >/dev/null 2>&1; then
        echo -e "${YELLOW}Killing existing aspen-node processes from previous runs${NC}"
        pkill -f "aspen-node" 2>/dev/null || true
        sleep 1  # Give processes time to exit
    fi

    # Clean and recreate data directory
    # This ensures no stale redb lock files prevent startup
    if [[ -d "$DATA_DIR" ]]; then
        echo -e "${YELLOW}Cleaning previous cluster data in $DATA_DIR${NC}"
        rm -rf "$DATA_DIR"
    fi
    mkdir -p "$DATA_DIR"

    # Start all nodes
    for ((i=1; i<=NODE_COUNT; i++)); do
        if ! start_node "$i"; then
            echo -e "${RED}Failed to start node $i${NC}" >&2
            exit 1
        fi
    done

    # Initialize cluster unless disabled
    if [[ -z "$NO_INIT" ]]; then
        sleep 1  # Give nodes a moment to stabilize
        if ! init_cluster; then
            echo -e "${YELLOW}Cluster initialization may need manual intervention${NC}"
        fi
    fi

    # Print cluster info
    print_info

    # Check initial health
    sleep 2
    check_health

    # Get cluster ticket for Iroh P2P connections
    echo -e "\n${BLUE}Cluster Ticket (for Iroh P2P):${NC}"
    local ticket
    ticket=$(curl -s "http://127.0.0.1:$BASE_HTTP/cluster-ticket" 2>/dev/null | grep -o '"ticket":"[^"]*"' | cut -d'"' -f4 || echo "unavailable")
    if [[ -n "$ticket" ]] && [[ "$ticket" != "unavailable" ]]; then
        echo "  $ticket"
        echo ""
        echo "Connect via Iroh:"
        echo "  nix run .#aspen-tui -- --ticket \"$ticket\""
    fi

    echo -e "\n${GREEN}Cluster is running. Press Ctrl+C to stop.${NC}"
    echo ""

    # Wait forever (until interrupted)
    while true; do
        # Periodically check if nodes are still running
        for pid in "${PIDS[@]}"; do
            if ! kill -0 "$pid" 2>/dev/null; then
                echo -e "${RED}Node (PID: $pid) died unexpectedly${NC}" >&2
                exit 1
            fi
        done
        sleep 5
    done
}

main "$@"
