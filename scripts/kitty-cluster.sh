#!/usr/bin/env bash
# Kitty terminal cluster launcher for Aspen
# Spawns N nodes in separate tabs within a single kitty window
#
# Usage: nix run .#kitty-cluster
#
# Environment variables:
#   ASPEN_NODE_COUNT  - Number of nodes to spawn (default: 4)
#   ASPEN_COOKIE      - Cluster authentication cookie (default: kitty-cluster-$$)
#   ASPEN_LOG_LEVEL   - Log level for nodes (default: info)
#   ASPEN_DATA_DIR    - Base data directory (default: /tmp/aspen-kitty-$$)
#   ASPEN_STORAGE     - Storage backend: inmemory, redb, sqlite (default: redb)
#   ASPEN_BLOBS       - Enable blob storage: true/false (default: true)
#   ASPEN_DOCS        - Enable iroh-docs CRDT sync: true/false (default: true)
#   ASPEN_DNS         - Enable DNS server on node 1: true/false (default: true)
#   ASPEN_DNS_ZONES   - DNS zones to serve (default: aspen.local)
#   ASPEN_DNS_PORT    - DNS server port (default: 15353, avoids mDNS port 5353)
#   ASPEN_DHT         - Enable DHT content discovery: true/false (default: true)
#   ASPEN_DHT_PORT    - Base DHT port (incremented per node, default: 6881)

set -eu

# Configuration with defaults
NODE_COUNT="${ASPEN_NODE_COUNT:-4}"
COOKIE="${ASPEN_COOKIE:-kitty-cluster-$$}"
LOG_LEVEL="${ASPEN_LOG_LEVEL:-info}"
DATA_DIR="${ASPEN_DATA_DIR:-/tmp/aspen-kitty-$$}"
STORAGE="${ASPEN_STORAGE:-redb}"  # redb uses DataFusion SQL layer
BLOBS_ENABLED="${ASPEN_BLOBS:-true}"  # Enable blob storage by default
DOCS_ENABLED="${ASPEN_DOCS:-true}"    # Enable iroh-docs CRDT sync by default
DNS_ENABLED="${ASPEN_DNS:-true}"      # Enable DNS server on node 1 by default
DNS_ZONES="${ASPEN_DNS_ZONES:-aspen.local}"  # DNS zones to serve
DNS_PORT="${ASPEN_DNS_PORT:-15353}"   # DNS port (default 15353 to avoid mDNS conflicts)
DHT_ENABLED="${ASPEN_DHT:-true}"      # Enable DHT content discovery by default
DHT_BASE_PORT="${ASPEN_DHT_PORT:-6881}"  # Base DHT port (incremented per node)

# Note: Docs namespace secret is now automatically derived from the cookie in aspen-node.
# You can override with ASPEN_DOCS_NAMESPACE_SECRET if needed for compatibility.

# Find binary in common locations
find_binary() {
    local name="$1"
    local bin=""

    # Check environment variable first
    local env_var="ASPEN_${name^^}_BIN"
    env_var="${env_var//-/_}"  # Replace - with _
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

    # Check target/release (cargo build --release)
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    local project_dir
    project_dir="$(dirname "$script_dir")"

    bin="$project_dir/target/release/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check target/debug (cargo build)
    bin="$project_dir/target/debug/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check nix result symlink
    bin="$project_dir/result/bin/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    echo ""
}

# Binary paths - auto-detect from env, PATH, or cargo build output
ASPEN_NODE_BIN="${ASPEN_NODE_BIN:-$(find_binary aspen-node)}"
ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"
ASPEN_TUI_BIN="${ASPEN_TUI_BIN:-$(find_binary aspen-tui)}"

# Kitty process ID (set after launch)
KITTY_PID=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Cleanup function
cleanup() {
    printf "\n${YELLOW}Shutting down cluster...${NC}\n"

    # Kill kitty process if running
    if [ -n "$KITTY_PID" ] && kill -0 "$KITTY_PID" 2>/dev/null; then
        kill "$KITTY_PID" 2>/dev/null || true
        # Wait briefly for graceful shutdown
        sleep 1
        # Force kill if still running
        if kill -0 "$KITTY_PID" 2>/dev/null; then
            kill -9 "$KITTY_PID" 2>/dev/null || true
        fi
    fi

    printf "${GREEN}Cluster stopped${NC}\n"
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

    if [ -z "$ASPEN_TUI_BIN" ] || [ ! -x "$ASPEN_TUI_BIN" ]; then
        printf "${RED}Error: aspen-tui binary not found${NC}\n" >&2
        missing=1
    fi

    if [ "$missing" -eq 1 ]; then
        printf "\n" >&2
        printf "Build with one of:\n" >&2
        printf "  cargo build --release --bin aspen-node --bin aspen-cli\n" >&2
        printf "  cargo build --release --bin aspen-tui --features tui\n" >&2
        printf "\n" >&2
        printf "Or use: nix run .#kitty-cluster\n" >&2
        exit 1
    fi

    if ! command -v kitty >/dev/null 2>&1; then
        printf "${RED}Error: kitty terminal not found${NC}\n" >&2
        printf "Install kitty or ensure it's in PATH\n" >&2
        exit 1
    fi
}

# Generate kitty session file
generate_session_file() {
    local session_file="$DATA_DIR/session.conf"

    # Start with empty file
    : > "$session_file"

    local id=1
    while [ "$id" -le "$NODE_COUNT" ]; do
        local node_data_dir="$DATA_DIR/node$id"

        # Generate deterministic secret key (64 hex chars = 32 bytes)
        local secret
        secret=$(printf '%064x' "$((1000 + id))")

        # Calculate DHT port for this node (each node gets unique port)
        local dht_port=$((DHT_BASE_PORT + id - 1))

        # First tab uses 'launch', subsequent tabs use 'new_tab'
        # Note: Docs namespace secret is automatically derived from cookie
        # DNS server only runs on node 1 to avoid port conflicts
        if [ "$id" -eq 1 ]; then
            cat >> "$session_file" << EOF
title node-$id
cd $node_data_dir
launch --hold sh -c 'RUST_LOG=$LOG_LEVEL ASPEN_BLOBS_ENABLED=$BLOBS_ENABLED ASPEN_DOCS_ENABLED=$DOCS_ENABLED ASPEN_DNS_SERVER_ENABLED=$DNS_ENABLED ASPEN_DNS_SERVER_ZONES=$DNS_ZONES ASPEN_DNS_SERVER_BIND_ADDR=127.0.0.1:$DNS_PORT ASPEN_CONTENT_DISCOVERY_ENABLED=$DHT_ENABLED ASPEN_CONTENT_DISCOVERY_SERVER_MODE=$DHT_ENABLED ASPEN_CONTENT_DISCOVERY_DHT_PORT=$dht_port ASPEN_CONTENT_DISCOVERY_AUTO_ANNOUNCE=$DHT_ENABLED exec "$ASPEN_NODE_BIN" --node-id $id --cookie "$COOKIE" --data-dir "$node_data_dir" --storage-backend "$STORAGE" --iroh-secret-key "$secret" 2>&1 | tee node.log'

EOF
        else
            cat >> "$session_file" << EOF
new_tab node-$id
cd $node_data_dir
launch --hold sh -c 'RUST_LOG=$LOG_LEVEL ASPEN_BLOBS_ENABLED=$BLOBS_ENABLED ASPEN_DOCS_ENABLED=$DOCS_ENABLED ASPEN_CONTENT_DISCOVERY_ENABLED=$DHT_ENABLED ASPEN_CONTENT_DISCOVERY_SERVER_MODE=$DHT_ENABLED ASPEN_CONTENT_DISCOVERY_DHT_PORT=$dht_port ASPEN_CONTENT_DISCOVERY_AUTO_ANNOUNCE=$DHT_ENABLED exec "$ASPEN_NODE_BIN" --node-id $id --cookie "$COOKIE" --data-dir "$node_data_dir" --storage-backend "$STORAGE" --iroh-secret-key "$secret" 2>&1 | tee node.log'

EOF
        fi

        id=$((id + 1))
    done

    # Add TUI tab - waits for ticket file then launches
    # Create a launcher script that the TUI tab will run
    local tui_launcher="$DATA_DIR/tui-launcher.sh"
    cat > "$tui_launcher" << 'LAUNCHER'
#!/bin/sh
echo "Waiting for cluster to initialize..."
TICKET_FILE="$1"
TUI_BIN="$2"
elapsed=0
while [ ! -f "$TICKET_FILE" ] && [ "$elapsed" -lt 60 ]; do
    sleep 1
    elapsed=$((elapsed + 1))
done
if [ -f "$TICKET_FILE" ]; then
    # Read ticket and strip any whitespace
    TICKET=$(tr -d '[:space:]' < "$TICKET_FILE")
    echo "Connecting to cluster..."
    echo "Ticket length: ${#TICKET}"
    echo "Ticket prefix: $(echo "$TICKET" | head -c 20)..."
    echo ""

    # Run TUI and capture exit code
    "$TUI_BIN" --ticket "$TICKET"
    EXIT_CODE=$?

    echo ""
    echo "TUI exited with code: $EXIT_CODE"
    echo "Press Enter to close this tab..."
    read
else
    echo "Timeout waiting for cluster ticket"
    echo "Press Enter to exit"
    read
fi
LAUNCHER
    chmod +x "$tui_launcher"

    cat >> "$session_file" << EOF
new_tab tui
cd $DATA_DIR
launch --hold $tui_launcher $DATA_DIR/ticket.txt $ASPEN_TUI_BIN

EOF

    # Focus TUI tab (last tab)
    # Note: focus_tab command may not be available in all kitty versions
    # echo "focus_tab $NODE_COUNT" >> "$session_file"

    echo "$session_file"
}

# Wait for cluster ticket from node 1 log
wait_for_ticket() {
    local log_file="$DATA_DIR/node1/node.log"
    local timeout=30
    local elapsed=0

    printf "  Waiting for node 1 to start" >&2

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$log_file" ]; then
            # Look for the ticket pattern in the log output
            # The ticket format is: aspen{base32} - all lowercase letters and digits 2-7
            local ticket
            ticket=$(grep -oE 'aspen[a-z2-7]{50,200}' "$log_file" 2>/dev/null | head -1 || true)

            if [ -n "$ticket" ]; then
                printf " ${GREEN}done${NC}\n" >&2
                # Return only the ticket on stdout
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

# Get endpoint ID from a node's log file
# Strips ANSI color codes before searching
get_endpoint_id() {
    local log_file="$1"
    # Strip ANSI escape sequences, then extract endpoint_id
    sed 's/\x1b\[[0-9;]*m//g' "$log_file" 2>/dev/null | \
        grep -oE 'endpoint_id=[a-f0-9]{64}' | head -1 | cut -d= -f2 || true
}

# Initialize the cluster with all nodes
init_cluster() {
    local ticket="$1"

    printf "  Waiting for nodes to discover each other..."

    # Wait for gossip discovery between nodes (they need to find each other first)
    sleep 5
    printf " ${GREEN}done${NC}\n"

    # Step 1: Initialize node 1 as single-node cluster
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

    # Give cluster a moment to stabilize
    sleep 2

    # Step 2: Add other nodes as learners
    if [ "$NODE_COUNT" -gt 1 ]; then
        printf "  Adding nodes 2-$NODE_COUNT as learners...\n"

        local id=2
        while [ "$id" -le "$NODE_COUNT" ]; do
            local endpoint_id
            endpoint_id=$(get_endpoint_id "$DATA_DIR/node$id/node.log")

            if [ -n "$endpoint_id" ]; then
                printf "    Node $id (endpoint: ${endpoint_id:0:16}...): "
                local output
                if output=$("$ASPEN_CLI_BIN" --ticket "$ticket" cluster add-learner \
                    --node-id "$id" --addr "$endpoint_id" 2>&1); then
                    printf "${GREEN}added${NC}\n"
                else
                    printf "${RED}failed${NC} - $output\n"
                fi
            else
                printf "    Node $id: ${RED}no endpoint ID found${NC}\n"
            fi
            sleep 1
            id=$((id + 1))
        done

        # Step 3: Change membership to include all nodes as voters
        printf "  Promoting all nodes to voters..."

        # Build list of all node IDs
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
        local output
        if output=$("$ASPEN_CLI_BIN" --ticket "$ticket" cluster change-membership $members 2>&1); then
            printf " ${GREEN}done${NC}\n"
        else
            printf " ${RED}failed${NC}\n"
            printf "    Error: $output\n"
        fi
    fi

    # Give cluster time to stabilize
    sleep 2
    return 0
}

# Print connection info
print_info() {
    local ticket="$1"

    printf "\n${BLUE}======================================${NC}\n"
    printf "${GREEN}Aspen Cluster Ready${NC} ($NODE_COUNT nodes)\n"
    printf "${BLUE}======================================${NC}\n"
    printf "\n"
    printf "Cookie:   $COOKIE\n"
    printf "Storage:  $STORAGE\n"
    printf "Blobs:    $BLOBS_ENABLED\n"
    printf "Docs:     $DOCS_ENABLED (namespace derived from cookie)\n"
    printf "DNS:      $DNS_ENABLED (zones: $DNS_ZONES, port: $DNS_PORT)\n"
    printf "DHT:      $DHT_ENABLED (ports: $DHT_BASE_PORT-$((DHT_BASE_PORT + NODE_COUNT - 1)))\n"
    printf "Data dir: $DATA_DIR\n"
    printf "\n"
    printf "${BLUE}Connect with TUI:${NC}\n"
    printf "  nix run .#aspen-tui -- --ticket $ticket\n"
    printf "\n"
    printf "${BLUE}Connect with CLI:${NC}\n"
    printf "  nix run .#aspen-cli -- --ticket $ticket cluster status\n"
    printf "  nix run .#aspen-cli -- --ticket $ticket kv set mykey 'hello'\n"
    printf "  nix run .#aspen-cli -- --ticket $ticket kv get mykey\n"
    if [ "$DOCS_ENABLED" = "true" ]; then
        printf "\n"
        printf "${BLUE}Docs (CRDT sync):${NC}\n"
        printf "  nix run .#aspen-cli -- --ticket $ticket docs status\n"
        printf "  nix run .#aspen-cli -- --ticket $ticket docs set mykey 'hello'\n"
        printf "  nix run .#aspen-cli -- --ticket $ticket docs list\n"
    fi
    if [ "$DNS_ENABLED" = "true" ]; then
        printf "\n"
        printf "${BLUE}DNS (zone: $DNS_ZONES):${NC}\n"
        printf "  nix run .#aspen-cli -- --ticket $ticket dns set api.$DNS_ZONES A 192.168.1.100\n"
        printf "  nix run .#aspen-cli -- --ticket $ticket dns get api.$DNS_ZONES A\n"
        printf "  nix run .#aspen-cli -- --ticket $ticket dns scan\n"
        printf "  dig @127.0.0.1 -p $DNS_PORT api.$DNS_ZONES A\n"
    fi
    if [ "$BLOBS_ENABLED" = "true" ]; then
        printf "\n"
        printf "${BLUE}Blob storage:${NC}\n"
        printf "  nix run .#aspen-cli -- --ticket $ticket blob add /path/to/file\n"
        printf "  nix run .#aspen-cli -- --ticket $ticket blob list\n"
        if [ "$DHT_ENABLED" = "true" ]; then
            printf "  nix run .#aspen-cli -- --ticket $ticket blob download-by-hash <hash>  # DHT lookup\n"
        fi
    fi
    printf "\n"
    printf "${BLUE}======================================${NC}\n"
    printf "Kitty window has $((NODE_COUNT + 1)) tabs:\n"
    printf "  node-1 through node-$NODE_COUNT (cluster nodes)\n"
    printf "  tui (interactive interface)\n"
    printf "Press Ctrl+C here to stop all nodes\n"
    printf "${BLUE}======================================${NC}\n"
}

# Main execution
main() {
    printf "${BLUE}Aspen Kitty Cluster${NC} - Starting $NODE_COUNT nodes\n"
    printf "\n"

    # Check prerequisites
    check_prerequisites

    printf "Using aspen-node: $ASPEN_NODE_BIN\n"
    printf "Using aspen-cli:  $ASPEN_CLI_BIN\n"
    printf "\n"

    # Clean up any existing data directory
    if [ -d "$DATA_DIR" ]; then
        printf "${YELLOW}Cleaning previous cluster data in $DATA_DIR${NC}\n"
        rm -rf "$DATA_DIR"
    fi

    # Create data directories for each node
    mkdir -p "$DATA_DIR"
    local id=1
    while [ "$id" -le "$NODE_COUNT" ]; do
        mkdir -p "$DATA_DIR/node$id"
        id=$((id + 1))
    done

    # Generate session file
    local session_file
    session_file=$(generate_session_file)
    printf "Generated session file: $session_file\n"
    printf "\n"

    # Launch kitty with the session
    printf "${BLUE}Launching kitty with $((NODE_COUNT + 1)) tabs ($NODE_COUNT nodes + TUI)...${NC}\n"
    kitty --session "$session_file" \
          --title "Aspen Cluster ($NODE_COUNT nodes)" \
          --override "tab_bar_style=powerline" &
    KITTY_PID=$!

    printf "  Kitty PID: $KITTY_PID\n"
    printf "\n"

    # Wait for the ticket from node 1
    printf "${BLUE}Waiting for cluster to start...${NC}\n"
    local ticket
    if ! ticket=$(wait_for_ticket); then
        printf "${RED}Failed to get cluster ticket. Check node logs in kitty.${NC}\n" >&2
        printf "Logs are at: $DATA_DIR/node*/node.log\n" >&2
        # Don't exit - let user inspect the kitty window
        wait "$KITTY_PID" 2>/dev/null || true
        exit 1
    fi

    # Initialize the cluster
    if ! init_cluster "$ticket"; then
        printf "${YELLOW}Cluster initialization failed. TUI may not work correctly.${NC}\n"
        printf "You can try manually: aspen-cli --ticket $ticket cluster init\n"
    fi

    # Write ticket file for TUI tab to pick up
    echo "$ticket" > "$DATA_DIR/ticket.txt"
    printf "  TUI tab connecting...\n"

    # Print connection info
    print_info "$ticket"

    # Wait for kitty to exit (user closes window)
    wait "$KITTY_PID" 2>/dev/null || true
}

main "$@"
