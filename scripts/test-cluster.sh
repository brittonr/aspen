#!/usr/bin/env bash
# Test cluster launcher for Aspen - optimized for CLI testing
# Based on kitty-cluster.sh but runs in background for CLI usage
#
# Usage: ./scripts/test-cluster.sh
#
# Environment variables:
#   ASPEN_NODE_COUNT  - Number of nodes to spawn (default: 3)
#   ASPEN_LOG_LEVEL   - Log level for nodes (default: info)
#   ASPEN_DATA_DIR    - Base data directory (default: /tmp/aspen-test-$$)
#   ASPEN_STORAGE     - Storage backend: inmemory, redb, sqlite (default: redb)
#   ASPEN_WORKERS     - Enable job workers (default: true)
#   ASPEN_VM_EXECUTOR - Enable VM executor (default: true)

set -eu

# Configuration with defaults
NODE_COUNT="${ASPEN_NODE_COUNT:-3}"
COOKIE="test-cluster-$$"
LOG_LEVEL="${ASPEN_LOG_LEVEL:-info}"
DATA_DIR="${ASPEN_DATA_DIR:-/tmp/aspen-test-$$}"
STORAGE="${ASPEN_STORAGE:-redb}"
WORKERS_ENABLED="${ASPEN_WORKERS:-true}"
VM_EXECUTOR_ENABLED="${ASPEN_VM_EXECUTOR:-true}"

# Resolve script directory and source shared functions
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/cluster-common.sh"

# Binary paths
ASPEN_NODE_BIN="${ASPEN_NODE_BIN:-$(find_binary aspen-node)}"
ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"

# Process PIDs
declare -a NODE_PIDS=()

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Cleanup function
cleanup() {
    printf "\n${YELLOW}Shutting down cluster...${NC}\n"

    # Kill all node processes
    for pid in "${NODE_PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            printf "  Stopping node (PID $pid)..."
            kill "$pid" 2>/dev/null || true
            printf " ${GREEN}done${NC}\n"
        fi
    done

    # Save the ticket for reference
    if [ -f "$DATA_DIR/ticket.txt" ]; then
        printf "\n${BLUE}Cluster ticket saved at:${NC} $DATA_DIR/ticket.txt\n"
    fi

    printf "${GREEN}Cluster stopped${NC}\n"
    printf "Data preserved at: $DATA_DIR\n"
}

trap cleanup EXIT INT TERM

# Validate prerequisites
check_prerequisites() {
    local missing=0

    if [ -z "$ASPEN_NODE_BIN" ] || [ ! -x "$ASPEN_NODE_BIN" ]; then
        printf "${RED}Error: aspen-node binary not found${NC}\n" >&2
        missing=1
    fi

    if [ -z "$ASPEN_CLI_BIN" ] || [ ! -x "$ASPEN_CLI_BIN" ]; then
        printf "${RED}Error: aspen-cli binary not found${NC}\n" >&2
        missing=1
    fi

    if [ "$missing" -eq 1 ]; then
        printf "\n" >&2
        printf "Build with:\n" >&2
        printf "  cargo build --release --bin aspen-node --bin aspen-cli\n" >&2
        exit 1
    fi
}

# Start a node
start_node() {
    local id=$1
    local node_data_dir="$DATA_DIR/node$id"
    local log_file="$node_data_dir/node.log"

    mkdir -p "$node_data_dir"

    # Generate deterministic secret key
    local secret
    secret=$(generate_secret_key "$id")

    # Note: Worker configuration is handled via config file, not CLI flags
    local worker_args=""

    printf "  Starting node $id..."

    # Start the node in background
    # Blob/docs/discovery are configured via environment variables
    RUST_LOG=$LOG_LEVEL \
    ASPEN_BLOBS_ENABLED=true \
    ASPEN_DOCS_ENABLED=true \
    ASPEN_CONTENT_DISCOVERY_ENABLED=true \
    "$ASPEN_NODE_BIN" \
        --node-id "$id" \
        --cookie "$COOKIE" \
        --data-dir "$node_data_dir" \
        --storage-backend "$STORAGE" \
        --iroh-secret-key "$secret" \
        $worker_args \
        > "$log_file" 2>&1 &

    local pid=$!
    NODE_PIDS+=("$pid")

    # Wait for node to start (strip ANSI codes before grepping)
    local timeout=30
    local elapsed=0
    while [ "$elapsed" -lt "$timeout" ]; do
        if sed 's/\x1b\[[0-9;]*m//g' "$log_file" 2>/dev/null | grep -q "endpoint_id="; then
            printf " ${GREEN}done${NC} (PID: $pid)\n"
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done

    printf " ${RED}timeout${NC}\n"
    return 1
}

# Wait for cluster ticket from node 1
wait_for_ticket() {
    local log_file="$DATA_DIR/node1/node.log"
    local timeout=30
    local elapsed=0

    printf "  Waiting for cluster ticket" >&2

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$log_file" ]; then
            # Look for the ticket pattern in the log (strip ANSI codes first)
            local ticket
            ticket=$(sed 's/\x1b\[[0-9;]*m//g' "$log_file" 2>/dev/null | grep -oE 'aspen[a-z2-7]{50,500}' | head -1 || true)

            if [ -n "$ticket" ]; then
                printf " ${GREEN}done${NC}\n" >&2
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

# Get endpoint ID from a node's log
get_endpoint_id() {
    local log_file="$1"
    sed 's/\x1b\[[0-9;]*m//g' "$log_file" 2>/dev/null | \
        grep -oE 'endpoint_id=[a-f0-9]{64}' | head -1 | cut -d= -f2 || true
}

# Initialize the cluster
init_cluster() {
    local ticket="$1"

    printf "\n${BLUE}Initializing cluster...${NC}\n"

    # Wait for nodes to discover each other
    printf "  Waiting for gossip discovery..."
    sleep 3
    printf " ${GREEN}done${NC}\n"

    # Initialize cluster on node 1
    printf "  Initializing node 1 as leader..."

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

    # Add other nodes if we have more than 1
    if [ "$NODE_COUNT" -gt 1 ]; then
        printf "  Adding nodes as learners:\n"

        local id=2
        while [ "$id" -le "$NODE_COUNT" ]; do
            local endpoint_id
            endpoint_id=$(get_endpoint_id "$DATA_DIR/node$id/node.log")

            if [ -n "$endpoint_id" ]; then
                printf "    Node $id: "
                if "$ASPEN_CLI_BIN" --ticket "$ticket" cluster add-learner \
                    --node-id "$id" --addr "$endpoint_id" >/dev/null 2>&1; then
                    printf "${GREEN}added${NC}\n"
                else
                    printf "${RED}failed${NC}\n"
                fi
            else
                printf "    Node $id: ${RED}no endpoint ID${NC}\n"
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
        if "$ASPEN_CLI_BIN" --ticket "$ticket" cluster change-membership $members >/dev/null 2>&1; then
            printf " ${GREEN}done${NC}\n"
        else
            printf " ${RED}failed${NC}\n"
        fi
    fi

    sleep 2
    return 0
}

# Print usage info
print_info() {
    local ticket="$1"

    printf "\n${BLUE}======================================${NC}\n"
    printf "${GREEN}Aspen Test Cluster Ready${NC} ($NODE_COUNT nodes)\n"
    printf "${BLUE}======================================${NC}\n"
    printf "\n"
    printf "Storage:  $STORAGE\n"
    printf "Workers:  $WORKERS_ENABLED (VM executor: $VM_EXECUTOR_ENABLED)\n"
    printf "Data dir: $DATA_DIR\n"
    printf "Logs:     $DATA_DIR/node*/node.log\n"
    printf "\n"
    printf "${BLUE}Cluster Ticket:${NC}\n"
    printf "$ticket\n"
    printf "\n"
    printf "${BLUE}Set environment for easy CLI usage:${NC}\n"
    printf "  export ASPEN_TICKET='$ticket'\n"
    printf "\n"
    printf "${BLUE}CLI Examples:${NC}\n"
    printf "  # Cluster operations\n"
    printf "  aspen-cli cluster status\n"
    printf "  aspen-cli cluster metrics\n"
    printf "\n"
    printf "  # Key-value operations\n"
    printf "  aspen-cli kv set mykey 'hello world'\n"
    printf "  aspen-cli kv get mykey\n"
    printf "  aspen-cli kv scan\n"
    printf "\n"
    printf "  # Blob storage\n"
    printf "  aspen-cli blob add /path/to/file --tag my-file\n"
    printf "  aspen-cli blob list\n"
    printf "  aspen-cli blob get <hash> --output file.out\n"
    printf "\n"
    if [ "$WORKERS_ENABLED" = "true" ]; then
        printf "  # Job operations\n"
        printf "  aspen-cli job submit test_job '{\"data\":\"test\"}'\n"
        printf "  aspen-cli job list\n"
        printf "  aspen-cli job status <job-id> --follow\n"
        printf "  aspen-cli job stats\n"
        printf "\n"
        if [ "$VM_EXECUTOR_ENABLED" = "true" ]; then
            printf "  # VM job submission\n"
            printf "  aspen-cli job submit-vm /path/to/binary --input 'test data'\n"
            printf "  aspen-cli job result <job-id>\n"
            printf "\n"
        fi
    fi
    printf "${BLUE}Direct node access:${NC}\n"
    printf "  # Query specific nodes\n"
    printf "  aspen-cli --ticket '$ticket' cluster status\n"
    printf "\n"
    printf "${BLUE}Monitor logs:${NC}\n"
    printf "  tail -f $DATA_DIR/node1/node.log\n"
    printf "\n"
    printf "${BLUE}======================================${NC}\n"
    printf "Press Ctrl+C to stop the cluster\n"
    printf "${BLUE}======================================${NC}\n"
}

# Main execution
main() {
    printf "${BLUE}Aspen Test Cluster${NC} - Starting $NODE_COUNT nodes\n"
    printf "\n"

    # Check prerequisites
    check_prerequisites

    printf "Using aspen-node: $ASPEN_NODE_BIN\n"
    printf "Using aspen-cli:  $ASPEN_CLI_BIN\n"
    printf "\n"

    # Clean up any existing data
    if [ -d "$DATA_DIR" ]; then
        printf "${YELLOW}Cleaning previous cluster data${NC}\n"
        rm -rf "$DATA_DIR"
    fi
    mkdir -p "$DATA_DIR"

    # Start nodes
    printf "${BLUE}Starting nodes...${NC}\n"
    local id=1
    while [ "$id" -le "$NODE_COUNT" ]; do
        if ! start_node "$id"; then
            printf "${RED}Failed to start node $id${NC}\n"
            exit 1
        fi
        id=$((id + 1))
    done

    # Get ticket
    printf "\n${BLUE}Getting cluster ticket...${NC}\n"
    local ticket
    if ! ticket=$(wait_for_ticket); then
        printf "${RED}Failed to get cluster ticket${NC}\n"
        printf "Check logs: tail -f $DATA_DIR/node1/node.log\n"
        exit 1
    fi

    # Save ticket for reference
    echo "$ticket" > "$DATA_DIR/ticket.txt"

    # Initialize cluster
    if ! init_cluster "$ticket"; then
        printf "${YELLOW}Warning: Cluster initialization had issues${NC}\n"
        printf "You can try manually: aspen-cli --ticket '$ticket' cluster init\n"
    fi

    # Print usage info
    print_info "$ticket"

    # Keep running
    while true; do
        # Check if nodes are still running
        local alive=0
        for pid in "${NODE_PIDS[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                alive=$((alive + 1))
            fi
        done

        if [ "$alive" -eq 0 ]; then
            printf "\n${RED}All nodes have stopped${NC}\n"
            exit 1
        elif [ "$alive" -lt "$NODE_COUNT" ]; then
            printf "\n${YELLOW}Warning: Only $alive/$NODE_COUNT nodes running${NC}\n"
        fi

        sleep 5
    done
}

main "$@"
