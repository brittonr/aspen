#!/usr/bin/env bash
# Launch Aspen cluster and TUI in tmux panes
set -euo pipefail

# Configuration
NODE_COUNT="${ASPEN_NODE_COUNT:-3}"
BASE_HTTP="${ASPEN_BASE_HTTP:-21001}"
SESSION_NAME="aspen-cluster"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting Aspen cluster with TUI in tmux...${NC}"

# Check if tmux is installed
if ! command -v tmux &> /dev/null; then
    echo -e "${RED}Error: tmux is not installed${NC}"
    echo "Please install tmux or use 'nix run .#cluster-with-tui' in Kitty"
    exit 1
fi

# Kill existing session if it exists
if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
    echo -e "${YELLOW}Killing existing session '$SESSION_NAME'...${NC}"
    tmux kill-session -t "$SESSION_NAME"
    sleep 1
fi

# Function to wait for cluster to be ready
wait_for_cluster() {
    local max_attempts=30
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        if curl -s "http://127.0.0.1:$BASE_HTTP/health" 2>/dev/null | grep -q "healthy"; then
            return 0
        fi
        sleep 1
        ((attempt++))
    done
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

# Create new tmux session with cluster
echo -e "${BLUE}Creating tmux session '$SESSION_NAME'...${NC}"
tmux new-session -d -s "$SESSION_NAME" -n "cluster" \
    "nix run $(dirname "${BASH_SOURCE[0]}")/..#cluster; echo 'Cluster stopped. Press Enter to exit.'; read"

# Wait for cluster to be ready
echo -e "${YELLOW}Waiting for cluster to start...${NC}"
if ! wait_for_cluster; then
    echo -e "${RED}Failed to start cluster${NC}"
    tmux kill-session -t "$SESSION_NAME"
    exit 1
fi

echo -e "${GREEN}Cluster is ready!${NC}"

# Try to get the Iroh ticket
echo -e "${BLUE}Getting cluster ticket...${NC}"
TICKET=$(get_cluster_ticket)

# Determine connection method
if [ -n "$TICKET" ]; then
    echo -e "${GREEN}Got Iroh ticket, launching TUI with P2P connection...${NC}"
    TUI_CMD="nix run $(dirname "${BASH_SOURCE[0]}")/..#aspen-tui -- --ticket '$TICKET'"
    CONNECTION_TYPE="Iroh P2P"
else
    echo -e "${YELLOW}Using HTTP connection for TUI...${NC}"
    # Build HTTP node arguments
    NODE_ARGS=""
    for ((i=1; i<=NODE_COUNT; i++)); do
        NODE_ARGS="$NODE_ARGS --nodes http://127.0.0.1:$((BASE_HTTP + i - 1))"
    done
    TUI_CMD="nix run $(dirname "${BASH_SOURCE[0]}")/..#aspen-tui -- $NODE_ARGS"
    CONNECTION_TYPE="HTTP"
fi

# Create new window for TUI
echo -e "${BLUE}Starting TUI in new tmux window ($CONNECTION_TYPE)...${NC}"
tmux new-window -t "$SESSION_NAME" -n "tui" "$TUI_CMD; echo 'TUI exited. Press Enter to close.'; read"

# Create status window with helpful commands
tmux new-window -t "$SESSION_NAME" -n "shell" -c "$(dirname "${BASH_SOURCE[0]}")/.."

# Select the TUI window
tmux select-window -t "$SESSION_NAME:tui"

# Attach to session or provide instructions
if [ -t 0 ] && [ -z "${TMUX:-}" ]; then
    # We're in an interactive terminal and not already in tmux
    echo -e "${GREEN}Attaching to tmux session...${NC}"
    tmux attach-session -t "$SESSION_NAME"
else
    echo -e "${GREEN}Cluster and TUI are running in tmux session '$SESSION_NAME'${NC}"
    echo ""
    echo -e "${YELLOW}Tmux commands:${NC}"
    echo "  Attach to session:  tmux attach -t $SESSION_NAME"
    echo "  Switch windows:     Ctrl-b <number> (0=cluster, 1=tui, 2=shell)"
    echo "  Switch panes:       Ctrl-b <arrow>"
    echo "  Detach:            Ctrl-b d"
    echo "  Kill session:      tmux kill-session -t $SESSION_NAME"
    echo ""
    echo -e "${BLUE}Window layout:${NC}"
    echo "  Window 0 (cluster): Cluster logs"
    echo "  Window 1 (tui):     TUI interface"
    echo "  Window 2 (shell):   Shell in project directory"
fi
