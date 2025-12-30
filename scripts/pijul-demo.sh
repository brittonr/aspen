#!/usr/bin/env bash
# Aspen Pijul Demo
# Launches a cluster configured for Pijul (patch-based VCS) operations
#
# Usage: ./scripts/pijul-demo.sh [start|stop|demo|help]
#        nix run .#pijul-demo (if added to flake)
#
# This script starts an Aspen cluster with blobs enabled and demonstrates
# Pijul features: repositories, channels, changes, and P2P distribution.

set -euo pipefail

# Pijul requires blob storage for content-addressed changes
export ASPEN_BLOBS="${ASPEN_BLOBS:-true}"
export ASPEN_DOCS="${ASPEN_DOCS:-false}"
export ASPEN_STORAGE="${ASPEN_STORAGE:-redb}"
export ASPEN_NODE_COUNT="${ASPEN_NODE_COUNT:-1}"
export ASPEN_LOG_LEVEL="${ASPEN_LOG_LEVEL:-info}"
export ASPEN_DATA_DIR="${ASPEN_DATA_DIR:-/tmp/aspen-pijul}"
export ASPEN_FOREGROUND="${ASPEN_FOREGROUND:-false}"

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLUSTER_SCRIPT="$SCRIPT_DIR/cluster.sh"

# Verify cluster script exists
if [ ! -x "$CLUSTER_SCRIPT" ]; then
    printf "Error: cluster.sh not found at %s\n" "$CLUSTER_SCRIPT" >&2
    exit 1
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Find CLI binary
find_cli() {
    local bin=""

    if [ -n "${ASPEN_CLI_BIN:-}" ] && [ -x "$ASPEN_CLI_BIN" ]; then
        echo "$ASPEN_CLI_BIN"
        return 0
    fi

    bin=$(command -v aspen-cli 2>/dev/null || echo "")
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    local project_dir="$(dirname "$SCRIPT_DIR")"
    for path in target/release/aspen-cli target/debug/aspen-cli result/bin/aspen-cli; do
        bin="$project_dir/$path"
        if [ -x "$bin" ]; then
            echo "$bin"
            return 0
        fi
    done

    echo ""
}

ASPEN_CLI="${ASPEN_CLI_BIN:-$(find_cli)}"

# Check if CLI has pijul support
check_pijul_support() {
    if [ -z "$ASPEN_CLI" ] || [ ! -x "$ASPEN_CLI" ]; then
        return 1
    fi

    # Check if pijul subcommand exists
    if "$ASPEN_CLI" --help 2>&1 | grep -q "pijul"; then
        return 0
    else
        return 1
    fi
}

# Wait for cluster ticket
wait_for_ticket() {
    local ticket_file="$ASPEN_DATA_DIR/ticket.txt"
    local timeout=30
    local elapsed=0

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$ticket_file" ]; then
            cat "$ticket_file"
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done

    return 1
}

# Run CLI command
cli() {
    local ticket="$1"
    shift
    "$ASPEN_CLI" --ticket "$ticket" "$@"
}

# Print header
print_header() {
    printf "\n${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${CYAN}  Aspen Pijul${NC} - Distributed Patch-Based Version Control\n"
    printf "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"
}

# Start the pijul cluster
cmd_start() {
    print_header

    if [ -z "$ASPEN_CLI" ] || [ ! -x "$ASPEN_CLI" ]; then
        printf "${RED}Error: aspen-cli not found${NC}\n"
        printf "Build with: cargo build --release --bin aspen-cli --features pijul\n"
        exit 1
    fi

    if ! check_pijul_support; then
        printf "${RED}Error: aspen-cli was not built with pijul feature${NC}\n\n"
        printf "${YELLOW}The pijul feature is optional and requires libpijul (GPL-2.0).${NC}\n"
        printf "Rebuild with:\n"
        printf "  cargo build --release --bin aspen-cli --features pijul\n"
        printf "  cargo build --release --bin aspen-node --features pijul\n\n"
        printf "Or build everything:\n"
        printf "  cargo build --release --features pijul\n"
        exit 1
    fi

    printf "${BLUE}Configuration:${NC}\n"
    printf "  Nodes:    %s\n" "$ASPEN_NODE_COUNT"
    printf "  Storage:  %s\n" "$ASPEN_STORAGE"
    printf "  Blobs:    %s ${GREEN}(required for Pijul changes)${NC}\n" "$ASPEN_BLOBS"
    printf "  Data dir: %s\n" "$ASPEN_DATA_DIR"
    printf "  CLI:      %s\n" "$ASPEN_CLI"
    printf "\n"

    printf "${BLUE}Starting Pijul cluster...${NC}\n\n"

    # Start cluster in background
    "$CLUSTER_SCRIPT" start

    # Wait for ticket
    printf "\n${BLUE}Waiting for cluster to be ready...${NC}\n"
    local ticket
    if ! ticket=$(wait_for_ticket); then
        printf "${RED}Failed to get cluster ticket${NC}\n"
        exit 1
    fi

    printf "${GREEN}Cluster ready!${NC}\n\n"

    # Print usage info
    printf "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${CYAN}Pijul CLI Commands${NC}\n"
    printf "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"

    printf "${YELLOW}Environment setup:${NC}\n"
    printf "  export ASPEN_TICKET='%s'\n\n" "$ticket"

    printf "${YELLOW}Repository operations:${NC}\n"
    printf "  aspen-cli pijul repo init <name>           # Create a repository\n"
    printf "  aspen-cli pijul repo list                  # List all repositories\n"
    printf "  aspen-cli pijul repo info <repo-id>        # Get repository details\n"
    printf "\n"

    printf "${YELLOW}Channel (branch) operations:${NC}\n"
    printf "  aspen-cli pijul channel list <repo-id>     # List channels\n"
    printf "  aspen-cli pijul channel create <repo-id> <name>\n"
    printf "  aspen-cli pijul channel fork <repo-id> <src> <dst>\n"
    printf "  aspen-cli pijul channel delete <repo-id> <name>\n"
    printf "  aspen-cli pijul channel info <repo-id> <name>\n"
    printf "\n"

    printf "${YELLOW}Change operations:${NC}\n"
    printf "  aspen-cli pijul record <repo-id> -w /path -m 'message'\n"
    printf "  aspen-cli pijul apply <repo-id> <change-hash>\n"
    printf "  aspen-cli pijul log <repo-id> -c <channel> -n 20\n"
    printf "  aspen-cli pijul checkout <repo-id> -c <channel> -o /output\n"
    printf "\n"

    printf "${YELLOW}Blob storage:${NC}\n"
    printf "  aspen-cli blob add /path/to/file           # Store file content\n"
    printf "  aspen-cli blob list                        # List stored blobs\n"
    printf "  aspen-cli blob get <hash>                  # Retrieve blob\n"
    printf "\n"

    printf "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${CYAN}Quick Start Demo${NC}\n"
    printf "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"
    printf "Run the interactive demo:\n"
    printf "  %s demo\n\n" "$0"

    printf "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${CYAN}Cluster Management${NC}\n"
    printf "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"
    printf "  %s stop                     # Stop the cluster\n" "$0"
    printf "  %s status                   # Check cluster status\n" "$CLUSTER_SCRIPT"
    printf "  %s logs 1                   # View node 1 logs\n" "$CLUSTER_SCRIPT"
    printf "\n"
}

# Stop the cluster
cmd_stop() {
    printf "${BLUE}Stopping Pijul cluster...${NC}\n"
    "$CLUSTER_SCRIPT" stop
}

# Run interactive demo
cmd_demo() {
    print_header

    if ! check_pijul_support; then
        printf "${RED}Error: aspen-cli was not built with pijul feature${NC}\n\n"
        printf "${YELLOW}The pijul feature is optional and requires libpijul (GPL-2.0).${NC}\n"
        printf "Rebuild with:\n"
        printf "  cargo build --release --bin aspen-cli --features pijul\n"
        exit 1
    fi

    local ticket_file="$ASPEN_DATA_DIR/ticket.txt"
    if [ ! -f "$ticket_file" ]; then
        printf "${RED}No running cluster found.${NC}\n"
        printf "Start one first: %s start\n" "$0"
        exit 1
    fi

    local ticket
    ticket=$(cat "$ticket_file")

    printf "${CYAN}Running Pijul Demo${NC}\n\n"
    printf "Ticket: %s\n\n" "${ticket:0:40}..."

    # Ensure cluster is initialized
    printf "${BLUE}Ensuring cluster is initialized...${NC}\n"
    if cli "$ticket" cluster init 2>&1 | grep -qE "initialized|already"; then
        printf "  ${GREEN}Cluster ready${NC}\n\n"
    else
        printf "  ${YELLOW}Cluster may need a moment...${NC}\n"
        sleep 2
        cli "$ticket" cluster init 2>&1 | grep -v WARN || true
        printf "\n"
    fi

    # Step 1: Create a repository
    printf "${BLUE}Step 1: Create a Pijul repository${NC}\n"
    printf "  \$ aspen-cli pijul repo init my-pijul-project\n"
    local output
    if output=$(cli "$ticket" pijul repo init my-pijul-project 2>&1); then
        printf "${GREEN}%s${NC}\n\n" "$output"
        # Extract repo ID from output
        local repo_id
        repo_id=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")

        if [ -n "$repo_id" ]; then
            # Step 2: Show repository info
            printf "${BLUE}Step 2: Show repository info${NC}\n"
            printf "  \$ aspen-cli pijul repo info %s\n" "${repo_id:0:16}..."
            cli "$ticket" pijul repo info "$repo_id" 2>&1 || true
            printf "\n"

            # Step 3: List channels
            printf "${BLUE}Step 3: List channels (default 'main' channel created)${NC}\n"
            printf "  \$ aspen-cli pijul channel list %s\n" "${repo_id:0:16}..."
            cli "$ticket" pijul channel list "$repo_id" 2>&1 || true
            printf "\n"

            # Step 4: Create a new channel
            printf "${BLUE}Step 4: Create a 'develop' channel${NC}\n"
            printf "  \$ aspen-cli pijul channel create %s develop\n" "${repo_id:0:16}..."
            cli "$ticket" pijul channel create "$repo_id" develop 2>&1 || true
            printf "\n"

            # Step 5: Fork a channel
            printf "${BLUE}Step 5: Fork 'main' to 'feature-x'${NC}\n"
            printf "  \$ aspen-cli pijul channel fork %s main feature-x\n" "${repo_id:0:16}..."
            cli "$ticket" pijul channel fork "$repo_id" main feature-x 2>&1 || true
            printf "\n"

            # Step 6: List channels again
            printf "${BLUE}Step 6: List all channels${NC}\n"
            printf "  \$ aspen-cli pijul channel list %s\n" "${repo_id:0:16}..."
            cli "$ticket" pijul channel list "$repo_id" 2>&1 || true
            printf "\n"

            # Step 7: Get channel info
            printf "${BLUE}Step 7: Get 'develop' channel info${NC}\n"
            printf "  \$ aspen-cli pijul channel info %s develop\n" "${repo_id:0:16}..."
            cli "$ticket" pijul channel info "$repo_id" develop 2>&1 || true
            printf "\n"

            # Step 8: Show log (empty initially)
            printf "${BLUE}Step 8: Show change log (empty initially)${NC}\n"
            printf "  \$ aspen-cli pijul log %s -c main\n" "${repo_id:0:16}..."
            cli "$ticket" pijul log "$repo_id" -c main 2>&1 || true
            printf "\n"

            # Step 9: List repositories
            printf "${BLUE}Step 9: List all repositories${NC}\n"
            printf "  \$ aspen-cli pijul repo list\n"
            cli "$ticket" pijul repo list 2>&1 || true
            printf "\n"

        else
            printf "${RED}Error: Could not extract repository ID from output${NC}\n"
            printf "Output was: %s\n\n" "$output"
        fi
    else
        printf "${YELLOW}%s${NC}\n\n" "$output"
    fi

    printf "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${GREEN}Demo complete!${NC}\n\n"
    printf "Explore more with:\n"
    printf "  export ASPEN_TICKET='%s'\n" "$ticket"
    printf "  aspen-cli pijul --help\n"
    printf "\n"
}

# Show help
cmd_help() {
    print_header

    printf "${YELLOW}Usage:${NC} %s [command]\n\n" "$0"

    printf "${YELLOW}Commands:${NC}\n"
    printf "  start   Start a Pijul-enabled Aspen cluster (default)\n"
    printf "  stop    Stop the running cluster\n"
    printf "  demo    Run an interactive demo of Pijul features\n"
    printf "  help    Show this help message\n"
    printf "\n"

    printf "${YELLOW}Environment Variables:${NC}\n"
    printf "  ASPEN_NODE_COUNT  Number of nodes (default: 1)\n"
    printf "  ASPEN_STORAGE     Storage backend: inmemory, redb (default: redb)\n"
    printf "  ASPEN_DATA_DIR    Data directory (default: /tmp/aspen-pijul)\n"
    printf "  ASPEN_LOG_LEVEL   Log level: trace, debug, info, warn (default: info)\n"
    printf "\n"

    printf "${YELLOW}Examples:${NC}\n"
    printf "  %s start                     # Start single-node Pijul cluster\n" "$0"
    printf "  %s demo                      # Run interactive demo\n" "$0"
    printf "  ASPEN_NODE_COUNT=3 %s start  # Start 3-node cluster\n" "$0"
    printf "  %s stop                      # Stop the cluster\n" "$0"
    printf "\n"

    printf "${YELLOW}About Pijul on Aspen:${NC}\n"
    printf "  Pijul is a patch-based distributed version control system.\n"
    printf "  Unlike Git's snapshot model, Pijul tracks changes as first-class\n"
    printf "  objects, enabling true cherry-picking and conflict-free merges.\n"
    printf "\n"
    printf "  Aspen-Pijul Architecture:\n"
    printf "  - Changes stored in iroh-blobs (content-addressed, BLAKE3)\n"
    printf "  - Channel refs stored with Raft consensus (linearizable)\n"
    printf "  - Pristine state reconstructed from change dependencies\n"
    printf "  - P2P sync via iroh-gossip announcements\n"
    printf "\n"
    printf "  Key Concepts:\n"
    printf "  - Repository: A collection of channels and changes\n"
    printf "  - Channel: Similar to a Git branch, points to a set of changes\n"
    printf "  - Change: An atomic patch with BLAKE3 hash identity\n"
    printf "  - Pristine: The reconstructed file tree from applied changes\n"
    printf "\n"

    printf "${YELLOW}Build Requirements:${NC}\n"
    printf "  The pijul feature requires libpijul (GPL-2.0-or-later).\n"
    printf "  Build with:\n"
    printf "    cargo build --release --features pijul\n"
    printf "\n"
}

# Main
main() {
    local cmd="${1:-start}"

    case "$cmd" in
        start)
            cmd_start
            ;;
        stop)
            cmd_stop
            ;;
        demo)
            cmd_demo
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
