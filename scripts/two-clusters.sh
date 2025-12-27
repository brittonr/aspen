#!/usr/bin/env bash
# Two-cluster seeding demo
# Starts two separate Aspen clusters that can discover and share blobs with each other
#
# Usage: ./scripts/two-clusters.sh [start|stop|status|demo]
#
# The clusters share content via:
#   - Gossip blob announcements (same network, different clusters)
#   - DHT discovery (if --features global-discovery is enabled)
#   - Direct Iroh connections (content-addressed, works across clusters)

set -eu

# Configuration
CLUSTER_A_NODES="${CLUSTER_A_NODES:-2}"
CLUSTER_B_NODES="${CLUSTER_B_NODES:-2}"
LOG_LEVEL="${ASPEN_LOG_LEVEL:-info}"
BASE_DATA_DIR="${ASPEN_DATA_DIR:-/tmp/aspen-two-clusters-$$}"
STORAGE="${ASPEN_STORAGE:-redb}"

# Cluster-specific settings
CLUSTER_A_COOKIE="cluster-alpha-$$"
CLUSTER_B_COOKIE="cluster-beta-$$"
CLUSTER_A_DIR="$BASE_DATA_DIR/cluster-a"
CLUSTER_B_DIR="$BASE_DATA_DIR/cluster-b"

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Find binary
find_binary() {
    local name="$1"
    local bin=""

    bin=$(command -v "$name" 2>/dev/null || echo "")
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    bin="$PROJECT_DIR/target/release/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    bin="$PROJECT_DIR/target/debug/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    bin="$PROJECT_DIR/result/bin/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    echo ""
}

ASPEN_NODE_BIN="${ASPEN_NODE_BIN:-$(find_binary aspen-node)}"
ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"

check_prerequisites() {
    if [ -z "$ASPEN_NODE_BIN" ] || [ ! -x "$ASPEN_NODE_BIN" ]; then
        printf "${RED}Error: aspen-node binary not found${NC}\n" >&2
        printf "Build with: cargo build --release --bin aspen-node\n" >&2
        exit 1
    fi

    if [ -z "$ASPEN_CLI_BIN" ] || [ ! -x "$ASPEN_CLI_BIN" ]; then
        printf "${RED}Error: aspen-cli binary not found${NC}\n" >&2
        printf "Build with: cargo build --release --bin aspen-cli\n" >&2
        exit 1
    fi
}

# Generate deterministic secret key
generate_secret_key() {
    local cluster="$1"
    local node_id="$2"
    if [ "$cluster" = "a" ]; then
        printf '%064x' "$((2000 + node_id))"
    else
        printf '%064x' "$((3000 + node_id))"
    fi
}

# Start a node in a cluster
start_node() {
    local cluster="$1"
    local node_id="$2"
    local cookie="$3"
    local data_dir="$4"
    local pid_file="$5"

    local node_data_dir="$data_dir/node$node_id"
    local secret_key
    secret_key=$(generate_secret_key "$cluster" "$node_id")
    local log_file="$node_data_dir/node.log"

    mkdir -p "$node_data_dir"

    # Enable blobs, docs, and content discovery for cross-cluster sharing
    RUST_LOG="$LOG_LEVEL" \
    ASPEN_BLOBS_ENABLED="true" \
    ASPEN_DOCS_ENABLED="true" \
    ASPEN_CONTENT_DISCOVERY_ENABLED="true" \
    ASPEN_CONTENT_DISCOVERY_AUTO_ANNOUNCE="true" \
    "$ASPEN_NODE_BIN" \
        --node-id "$node_id" \
        --cookie "$cookie" \
        --data-dir "$node_data_dir" \
        --storage-backend "$STORAGE" \
        --iroh-secret-key "$secret_key" \
        > "$log_file" 2>&1 &

    local pid=$!
    echo "$pid" >> "$pid_file"
    printf "    Node %d: PID %d\n" "$node_id" "$pid"
}

# Wait for ticket from node 1 log
wait_for_ticket() {
    local log_file="$1"
    local timeout=30
    local elapsed=0

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$log_file" ]; then
            local ticket
            ticket=$(grep -oE 'aspen[a-z2-7]{50,200}' "$log_file" 2>/dev/null | head -1 || true)
            if [ -n "$ticket" ]; then
                echo "$ticket"
                return 0
            fi
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    return 1
}

# Get endpoint ID from log
get_endpoint_id() {
    local log_file="$1"
    sed 's/\x1b\[[0-9;]*m//g' "$log_file" 2>/dev/null | \
        grep -oE 'endpoint_id=[a-f0-9]{64}' | head -1 | cut -d= -f2 || true
}

# Initialize a cluster
init_cluster() {
    local ticket="$1"
    local node_count="$2"
    local data_dir="$3"

    sleep 3

    # Initialize node 1
    local attempts=0
    while [ "$attempts" -lt 5 ]; do
        if "$ASPEN_CLI_BIN" --ticket "$ticket" cluster init >/dev/null 2>&1; then
            break
        fi
        attempts=$((attempts + 1))
        sleep 2
    done

    if [ "$attempts" -eq 5 ]; then
        return 1
    fi

    sleep 2

    # Add other nodes
    if [ "$node_count" -gt 1 ]; then
        local id=2
        while [ "$id" -le "$node_count" ]; do
            local endpoint_id
            endpoint_id=$(get_endpoint_id "$data_dir/node$id/node.log")
            if [ -n "$endpoint_id" ]; then
                "$ASPEN_CLI_BIN" --ticket "$ticket" cluster add-learner \
                    --node-id "$id" --addr "$endpoint_id" >/dev/null 2>&1 || true
            fi
            sleep 1
            id=$((id + 1))
        done

        # Promote to voters
        local members=""
        id=1
        while [ "$id" -le "$node_count" ]; do
            members="$members $id"
            id=$((id + 1))
        done

        sleep 2
        "$ASPEN_CLI_BIN" --ticket "$ticket" cluster change-membership $members >/dev/null 2>&1 || true
    fi

    return 0
}

# Start both clusters
cmd_start() {
    check_prerequisites

    printf "${BLUE}Two-Cluster Seeding Demo${NC}\n"
    printf "========================\n\n"

    mkdir -p "$BASE_DATA_DIR"

    # Start Cluster A
    printf "${CYAN}Starting Cluster A${NC} (%d nodes, cookie: %s)\n" "$CLUSTER_A_NODES" "$CLUSTER_A_COOKIE"
    mkdir -p "$CLUSTER_A_DIR"
    local pid_file_a="$CLUSTER_A_DIR/pids"
    > "$pid_file_a"

    local id=1
    while [ "$id" -le "$CLUSTER_A_NODES" ]; do
        start_node "a" "$id" "$CLUSTER_A_COOKIE" "$CLUSTER_A_DIR" "$pid_file_a"
        id=$((id + 1))
    done

    printf "  Waiting for Cluster A ticket..."
    local ticket_a
    if ticket_a=$(wait_for_ticket "$CLUSTER_A_DIR/node1/node.log"); then
        printf " ${GREEN}done${NC}\n"
        echo "$ticket_a" > "$CLUSTER_A_DIR/ticket.txt"
    else
        printf " ${RED}failed${NC}\n"
        exit 1
    fi

    printf "  Initializing Cluster A..."
    if init_cluster "$ticket_a" "$CLUSTER_A_NODES" "$CLUSTER_A_DIR"; then
        printf " ${GREEN}done${NC}\n"
    else
        printf " ${RED}failed${NC}\n"
    fi

    printf "\n"

    # Start Cluster B
    printf "${CYAN}Starting Cluster B${NC} (%d nodes, cookie: %s)\n" "$CLUSTER_B_NODES" "$CLUSTER_B_COOKIE"
    mkdir -p "$CLUSTER_B_DIR"
    local pid_file_b="$CLUSTER_B_DIR/pids"
    > "$pid_file_b"

    id=1
    while [ "$id" -le "$CLUSTER_B_NODES" ]; do
        start_node "b" "$id" "$CLUSTER_B_COOKIE" "$CLUSTER_B_DIR" "$pid_file_b"
        id=$((id + 1))
    done

    printf "  Waiting for Cluster B ticket..."
    local ticket_b
    if ticket_b=$(wait_for_ticket "$CLUSTER_B_DIR/node1/node.log"); then
        printf " ${GREEN}done${NC}\n"
        echo "$ticket_b" > "$CLUSTER_B_DIR/ticket.txt"
    else
        printf " ${RED}failed${NC}\n"
        exit 1
    fi

    printf "  Initializing Cluster B..."
    if init_cluster "$ticket_b" "$CLUSTER_B_NODES" "$CLUSTER_B_DIR"; then
        printf " ${GREEN}done${NC}\n"
    else
        printf " ${RED}failed${NC}\n"
    fi

    # Print info
    printf "\n${BLUE}========================================${NC}\n"
    printf "${GREEN}Two Clusters Ready${NC}\n"
    printf "${BLUE}========================================${NC}\n\n"

    printf "${CYAN}Cluster A:${NC}\n"
    printf "  Nodes:  %d\n" "$CLUSTER_A_NODES"
    printf "  Cookie: %s\n" "$CLUSTER_A_COOKIE"
    printf "  Ticket: %s\n" "$ticket_a"
    printf "\n"

    printf "${CYAN}Cluster B:${NC}\n"
    printf "  Nodes:  %d\n" "$CLUSTER_B_NODES"
    printf "  Cookie: %s\n" "$CLUSTER_B_COOKIE"
    printf "  Ticket: %s\n" "$ticket_b"
    printf "\n"

    printf "${BLUE}Data directory:${NC} %s\n\n" "$BASE_DATA_DIR"

    printf "${BLUE}Demo commands:${NC}\n"
    printf "  # Store a blob in Cluster A\n"
    printf "  echo 'Hello from Cluster A!' | %s --ticket %s blobs add -\n\n" "$ASPEN_CLI_BIN" "$ticket_a"
    printf "  # Get blob hash, then fetch from Cluster B (cross-cluster)\n"
    printf "  %s --ticket %s blobs list\n" "$ASPEN_CLI_BIN" "$ticket_a"
    printf "  %s --ticket %s blobs get <hash>\n\n" "$ASPEN_CLI_BIN" "$ticket_b"

    printf "${BLUE}Stop clusters:${NC}\n"
    printf "  %s stop\n" "$0"
    printf "${BLUE}========================================${NC}\n"

    # Save state for stop command
    echo "$BASE_DATA_DIR" > "/tmp/aspen-two-clusters-datadir"
}

# Stop both clusters
cmd_stop() {
    printf "${BLUE}Stopping clusters...${NC}\n"

    # Find data dir
    local data_dir="$BASE_DATA_DIR"
    if [ -f "/tmp/aspen-two-clusters-datadir" ]; then
        data_dir=$(cat "/tmp/aspen-two-clusters-datadir")
    fi

    for cluster in "cluster-a" "cluster-b"; do
        local pid_file="$data_dir/$cluster/pids"
        if [ -f "$pid_file" ]; then
            printf "  Stopping %s: " "$cluster"
            local count=0
            while read -r pid; do
                if kill "$pid" 2>/dev/null; then
                    count=$((count + 1))
                fi
            done < "$pid_file"
            printf "${GREEN}%d processes${NC}\n" "$count"
            rm -f "$pid_file"
        fi
    done

    rm -f "/tmp/aspen-two-clusters-datadir"
    printf "${GREEN}Done${NC}\n"
}

# Show status
cmd_status() {
    local data_dir="$BASE_DATA_DIR"
    if [ -f "/tmp/aspen-two-clusters-datadir" ]; then
        data_dir=$(cat "/tmp/aspen-two-clusters-datadir")
    fi

    printf "${BLUE}Cluster Status${NC}\n\n"

    for cluster in "cluster-a" "cluster-b"; do
        local pid_file="$data_dir/$cluster/pids"
        local ticket_file="$data_dir/$cluster/ticket.txt"

        printf "${CYAN}%s:${NC}\n" "$cluster"

        if [ -f "$pid_file" ]; then
            local running=0
            local total=0
            while read -r pid; do
                total=$((total + 1))
                if kill -0 "$pid" 2>/dev/null; then
                    running=$((running + 1))
                fi
            done < "$pid_file"
            printf "  Nodes: %d/%d running\n" "$running" "$total"
        else
            printf "  Nodes: ${YELLOW}not started${NC}\n"
        fi

        if [ -f "$ticket_file" ]; then
            printf "  Ticket: %s\n" "$(cat "$ticket_file")"
        fi
        printf "\n"
    done
}

# Run demo
cmd_demo() {
    local data_dir="$BASE_DATA_DIR"
    if [ -f "/tmp/aspen-two-clusters-datadir" ]; then
        data_dir=$(cat "/tmp/aspen-two-clusters-datadir")
    fi

    local ticket_a ticket_b
    ticket_a=$(cat "$data_dir/cluster-a/ticket.txt" 2>/dev/null || echo "")
    ticket_b=$(cat "$data_dir/cluster-b/ticket.txt" 2>/dev/null || echo "")

    if [ -z "$ticket_a" ] || [ -z "$ticket_b" ]; then
        printf "${RED}Clusters not running. Start with: %s start${NC}\n" "$0"
        exit 1
    fi

    printf "${BLUE}Cross-Cluster Blob Seeding Demo${NC}\n"
    printf "================================\n\n"

    # Create test data
    local test_data="Hello from Cluster A! Timestamp: $(date)"
    printf "1. Creating blob in Cluster A...\n"
    printf "   Data: %s\n" "$test_data"

    local hash
    hash=$(echo "$test_data" | "$ASPEN_CLI_BIN" --ticket "$ticket_a" blobs add - 2>/dev/null | grep -oE '[a-z2-7]{52}' | head -1 || echo "")

    if [ -z "$hash" ]; then
        printf "   ${RED}Failed to add blob${NC}\n"
        exit 1
    fi

    printf "   ${GREEN}Blob hash: %s${NC}\n\n" "$hash"

    printf "2. Listing blobs in Cluster A...\n"
    "$ASPEN_CLI_BIN" --ticket "$ticket_a" blobs list 2>/dev/null | head -5
    printf "\n"

    printf "3. Fetching blob from Cluster B (cross-cluster)...\n"
    printf "   (This uses Iroh's content-addressed networking)\n"

    # Try to fetch - may need DHT or direct connection
    local fetched
    fetched=$("$ASPEN_CLI_BIN" --ticket "$ticket_b" blobs get "$hash" 2>/dev/null || echo "")

    if [ -n "$fetched" ]; then
        printf "   ${GREEN}Success!${NC}\n"
        printf "   Retrieved: %s\n" "$fetched"
    else
        printf "   ${YELLOW}Blob not yet discoverable across clusters${NC}\n"
        printf "   (May need DHT discovery or direct peer connection)\n"
        printf "\n"
        printf "   To enable cross-cluster discovery:\n"
        printf "   - Build with: cargo build --features global-discovery\n"
        printf "   - Or manually connect nodes via tickets\n"
    fi

    printf "\n${GREEN}Demo complete!${NC}\n"
}

# Main
case "${1:-start}" in
    start)
        cmd_start
        ;;
    stop)
        cmd_stop
        ;;
    status)
        cmd_status
        ;;
    demo)
        cmd_demo
        ;;
    *)
        printf "Usage: %s [start|stop|status|demo]\n" "$0"
        printf "\n"
        printf "Commands:\n"
        printf "  start   Start two clusters with blobs enabled\n"
        printf "  stop    Stop both clusters\n"
        printf "  status  Show cluster status and tickets\n"
        printf "  demo    Run cross-cluster blob seeding demo\n"
        printf "\n"
        printf "Environment variables:\n"
        printf "  CLUSTER_A_NODES  Nodes in cluster A (default: 2)\n"
        printf "  CLUSTER_B_NODES  Nodes in cluster B (default: 2)\n"
        printf "  ASPEN_LOG_LEVEL  Log level (default: info)\n"
        printf "  ASPEN_STORAGE    Storage backend (default: redb)\n"
        exit 1
        ;;
esac
