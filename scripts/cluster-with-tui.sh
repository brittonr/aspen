#!/usr/bin/env bash
# Launch Aspen cluster and TUI in separate Kitty windows
set -euo pipefail

# Check if we're running in Kitty
if [ -z "${KITTY_WINDOW_ID:-}" ]; then
    echo "Error: This script must be run from within Kitty terminal"
    echo "Please run this from a Kitty terminal window"
    exit 1
fi

# Configuration
CLUSTER_SCRIPT="${1:-$(dirname "${BASH_SOURCE[0]}")/cluster.sh}"
NODE_COUNT="${ASPEN_NODE_COUNT:-3}"
BASE_HTTP="${ASPEN_BASE_HTTP:-21001}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting Aspen cluster with TUI in Kitty...${NC}"

# Function to wait for cluster to be ready
wait_for_cluster() {
    local max_attempts=30
    local attempt=0

    echo -e "${YELLOW}Waiting for cluster to be ready...${NC}"

    while [ $attempt -lt $max_attempts ]; do
        if curl -s "http://127.0.0.1:$BASE_HTTP/health" 2>/dev/null | grep -q "healthy"; then
            echo -e "${GREEN}Cluster is ready!${NC}"
            return 0
        fi
        sleep 1
        ((attempt++))
        echo -n "."
    done

    echo -e "${RED}Timeout waiting for cluster to start${NC}"
    return 1
}

# Get the cluster ticket after it's ready
get_cluster_ticket() {
    local ticket_response
    local ticket

    # Get endpoint IDs from all nodes
    local endpoint_ids=""
    for ((i=1; i<=NODE_COUNT; i++)); do
        local port=$((BASE_HTTP + i - 1))
        local node_info
        node_info=$(curl -s "http://127.0.0.1:$port/node-info" 2>/dev/null || echo "{}")
        local endpoint_addr
        endpoint_addr=$(echo "$node_info" | jq -r '.endpoint_addr.id' 2>/dev/null || echo "")

        if [ -n "$endpoint_addr" ] && [ "$endpoint_addr" != "null" ]; then
            if [ -n "$endpoint_ids" ]; then
                endpoint_ids="${endpoint_ids},${endpoint_addr}"
            else
                endpoint_ids="${endpoint_addr}"
            fi
        fi
    done

    # Get combined ticket
    if [ -n "$endpoint_ids" ]; then
        ticket_response=$(curl -s "http://127.0.0.1:$BASE_HTTP/cluster-ticket-combined?endpoint_ids=$endpoint_ids" 2>/dev/null || echo "{}")
    else
        ticket_response=$(curl -s "http://127.0.0.1:$BASE_HTTP/cluster-ticket-combined" 2>/dev/null || echo "{}")
    fi

    ticket=$(echo "$ticket_response" | jq -r '.ticket' 2>/dev/null || echo "")

    if [ -n "$ticket" ] && [ "$ticket" != "null" ]; then
        echo "$ticket"
    else
        echo ""
    fi
}

# First, start the cluster in the current window
echo -e "${BLUE}Starting cluster in current window...${NC}"
echo -e "${YELLOW}Press Ctrl+C in this window to stop both cluster and TUI${NC}"
echo ""

# Start cluster in background of current shell
"$CLUSTER_SCRIPT" &
CLUSTER_PID=$!

# Trap to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}Shutting down...${NC}"
    # Kill cluster if still running
    if kill -0 $CLUSTER_PID 2>/dev/null; then
        kill $CLUSTER_PID 2>/dev/null || true
    fi
    # Close the TUI window if it exists
    if [ -n "${TUI_WINDOW_ID:-}" ]; then
        kitty @ close-window --match id:$TUI_WINDOW_ID 2>/dev/null || true
    fi
    exit 0
}
trap cleanup EXIT INT TERM

# Wait for cluster to be ready
if ! wait_for_cluster; then
    echo -e "${RED}Failed to start cluster${NC}"
    exit 1
fi

# Try to get the Iroh ticket
echo -e "${BLUE}Getting cluster ticket...${NC}"
TICKET=$(get_cluster_ticket)

# Determine connection method
if [ -n "$TICKET" ]; then
    echo -e "${GREEN}Got Iroh ticket, launching TUI with P2P connection...${NC}"
    CONNECTION_ARGS="--ticket \"$TICKET\""
    CONNECTION_TYPE="Iroh P2P"
else
    echo -e "${YELLOW}Using HTTP connection for TUI...${NC}"
    # Build HTTP node arguments
    CONNECTION_ARGS=""
    for ((i=1; i<=NODE_COUNT; i++)); do
        CONNECTION_ARGS="$CONNECTION_ARGS --nodes http://127.0.0.1:$((BASE_HTTP + i - 1))"
    done
    CONNECTION_TYPE="HTTP"
fi

# Launch TUI in a new Kitty window
echo -e "${BLUE}Launching TUI in new Kitty window ($CONNECTION_TYPE)...${NC}"

# Create the TUI command
TUI_CMD="nix run $(dirname "$CLUSTER_SCRIPT")/..#aspen-tui -- $CONNECTION_ARGS"

# Launch in new Kitty window and capture the window ID
TUI_WINDOW_ID=$(kitty @ launch \
    --type=os-window \
    --title="Aspen TUI" \
    --cwd="$(dirname "$CLUSTER_SCRIPT")/.." \
    bash -c "$TUI_CMD; echo 'TUI exited. Press Enter to close.'; read" \
    2>/dev/null | grep -o 'id:[0-9]*' | cut -d: -f2 || echo "")

if [ -n "$TUI_WINDOW_ID" ]; then
    echo -e "${GREEN}TUI launched in window ID: $TUI_WINDOW_ID${NC}"
else
    # Fallback: try launching in a new tab if new window fails
    echo -e "${YELLOW}Trying to launch TUI in new tab instead...${NC}"
    TUI_WINDOW_ID=$(kitty @ launch \
        --type=tab \
        --title="Aspen TUI" \
        --cwd="$(dirname "$CLUSTER_SCRIPT")/.." \
        bash -c "$TUI_CMD; echo 'TUI exited. Press Enter to close.'; read" \
        2>/dev/null | grep -o 'id:[0-9]*' | cut -d: -f2 || echo "")

    if [ -n "$TUI_WINDOW_ID" ]; then
        echo -e "${GREEN}TUI launched in tab ID: $TUI_WINDOW_ID${NC}"
    else
        echo -e "${YELLOW}Could not launch TUI in Kitty. You can manually run:${NC}"
        echo "  $TUI_CMD"
    fi
fi

echo -e "${GREEN}Cluster and TUI are running!${NC}"
echo -e "${YELLOW}Commands:${NC}"
echo "  - This window shows cluster logs"
echo "  - Switch to the TUI window to interact with the cluster"
echo "  - Press Ctrl+C here to stop everything"
echo ""

# Wait for cluster process
wait $CLUSTER_PID
