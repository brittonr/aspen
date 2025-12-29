#!/usr/bin/env bash
# Aspen Forge Demo
# Launches a cluster configured for Forge (decentralized Git) operations
#
# Usage: ./scripts/forge-demo.sh [start|stop|demo|help]
#        nix run .#forge-demo (if added to flake)
#
# This script starts an Aspen cluster with blobs enabled and demonstrates
# Forge features: repositories, branches, issues, patches, and tags.

set -eu

# Forge requires blob storage for content-addressed Git objects
export ASPEN_BLOBS="${ASPEN_BLOBS:-true}"
export ASPEN_DOCS="${ASPEN_DOCS:-false}"
export ASPEN_STORAGE="${ASPEN_STORAGE:-redb}"
export ASPEN_NODE_COUNT="${ASPEN_NODE_COUNT:-1}"
export ASPEN_LOG_LEVEL="${ASPEN_LOG_LEVEL:-info}"
export ASPEN_DATA_DIR="${ASPEN_DATA_DIR:-/tmp/aspen-forge}"
export ASPEN_FOREGROUND="${ASPEN_FOREGROUND:-false}"

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLUSTER_SCRIPT="$SCRIPT_DIR/cluster.sh"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
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
    printf "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${CYAN}  Aspen Forge${NC} - Decentralized Git on Aspen\n"
    printf "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"
}

# Start the forge cluster
cmd_start() {
    print_header

    if [ -z "$ASPEN_CLI" ] || [ ! -x "$ASPEN_CLI" ]; then
        printf "${RED}Error: aspen-cli not found${NC}\n"
        printf "Build with: cargo build --release --bin aspen-cli\n"
        exit 1
    fi

    printf "${BLUE}Configuration:${NC}\n"
    printf "  Nodes:    %s\n" "$ASPEN_NODE_COUNT"
    printf "  Storage:  %s\n" "$ASPEN_STORAGE"
    printf "  Blobs:    %s ${GREEN}(required for Forge)${NC}\n" "$ASPEN_BLOBS"
    printf "  Data dir: %s\n" "$ASPEN_DATA_DIR"
    printf "  CLI:      %s\n" "$ASPEN_CLI"
    printf "\n"

    printf "${BLUE}Starting Forge cluster...${NC}\n\n"

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
    printf "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${CYAN}Forge CLI Commands${NC}\n"
    printf "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"

    printf "${YELLOW}Environment setup:${NC}\n"
    printf "  export ASPEN_TICKET='%s'\n\n" "$ticket"

    printf "${YELLOW}Git operations:${NC}\n"
    printf "  aspen-cli git init <name>              # Create a repository\n"
    printf "  aspen-cli git show --repo <id>         # Show repository details\n"
    printf "  aspen-cli git commit --repo <id> ...   # Create a commit\n"
    printf "  aspen-cli git log --repo <id>          # Show commit history\n"
    printf "\n"

    printf "${YELLOW}Branch operations:${NC}\n"
    printf "  aspen-cli branch list --repo <id>      # List branches\n"
    printf "  aspen-cli branch create --repo <id> <name> --from <hash>\n"
    printf "  aspen-cli branch delete --repo <id> <name>\n"
    printf "\n"

    printf "${YELLOW}Tag operations:${NC}\n"
    printf "  aspen-cli tag list --repo <id>         # List tags\n"
    printf "  aspen-cli tag create --repo <id> <name> --commit <hash>\n"
    printf "  aspen-cli tag delete --repo <id> <name>\n"
    printf "\n"

    printf "${YELLOW}Issue tracking:${NC}\n"
    printf "  aspen-cli issue create --repo <id> -t 'Title' -b 'Body'\n"
    printf "  aspen-cli issue list --repo <id>       # List issues\n"
    printf "  aspen-cli issue show --repo <id> <issue-id>\n"
    printf "  aspen-cli issue comment --repo <id> <issue-id> -b 'Comment'\n"
    printf "  aspen-cli issue close --repo <id> <issue-id>\n"
    printf "\n"

    printf "${YELLOW}Patch management:${NC}\n"
    printf "  aspen-cli patch create --repo <id> -t 'Title' --base <hash> --head <hash>\n"
    printf "  aspen-cli patch list --repo <id>       # List patches\n"
    printf "  aspen-cli patch show --repo <id> <patch-id>\n"
    printf "  aspen-cli patch merge --repo <id> <patch-id> --commit <merge-hash>\n"
    printf "\n"

    printf "${YELLOW}Blob storage:${NC}\n"
    printf "  aspen-cli blob add /path/to/file       # Store file content\n"
    printf "  aspen-cli blob list                    # List stored blobs\n"
    printf "  aspen-cli blob get <hash>              # Retrieve blob\n"
    printf "\n"

    printf "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${CYAN}Quick Start Demo${NC}\n"
    printf "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"
    printf "Run the interactive demo:\n"
    printf "  %s demo\n\n" "$0"

    printf "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${CYAN}Cluster Management${NC}\n"
    printf "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"
    printf "  %s stop                     # Stop the cluster\n" "$0"
    printf "  %s status                   # Check cluster status\n" "$CLUSTER_SCRIPT"
    printf "  %s logs 1                   # View node 1 logs\n" "$CLUSTER_SCRIPT"
    printf "\n"
}

# Stop the cluster
cmd_stop() {
    printf "${BLUE}Stopping Forge cluster...${NC}\n"
    "$CLUSTER_SCRIPT" stop
}

# Run interactive demo
cmd_demo() {
    print_header

    local ticket_file="$ASPEN_DATA_DIR/ticket.txt"
    if [ ! -f "$ticket_file" ]; then
        printf "${RED}No running cluster found.${NC}\n"
        printf "Start one first: %s start\n" "$0"
        exit 1
    fi

    local ticket
    ticket=$(cat "$ticket_file")

    printf "${CYAN}Running Forge Demo${NC}\n\n"
    printf "Ticket: %s\n\n" "${ticket:0:40}..."

    # Ensure cluster is initialized
    printf "${BLUE}Ensuring cluster is initialized...${NC}\n"
    if cli "$ticket" cluster init 2>&1 | grep -q "initialized\|already"; then
        printf "  ${GREEN}Cluster ready${NC}\n\n"
    else
        printf "  ${YELLOW}Cluster may need a moment...${NC}\n"
        sleep 2
        cli "$ticket" cluster init 2>&1 | grep -v WARN || true
        printf "\n"
    fi

    # Step 1: Create a repository
    printf "${BLUE}Step 1: Create a repository${NC}\n"
    printf "  $ aspen-cli git init my-project\n"
    local output
    if output=$(cli "$ticket" git init my-project 2>&1); then
        printf "${GREEN}%s${NC}\n\n" "$output"
        # Extract repo ID from output
        local repo_id
        repo_id=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || echo "")

        if [ -n "$repo_id" ]; then
            # Step 2: Show repository info
            printf "${BLUE}Step 2: Show repository info${NC}\n"
            printf "  \$ aspen-cli git show --repo %s\n" "${repo_id:0:16}..."
            cli "$ticket" git show --repo "$repo_id" 2>&1 || true
            printf "\n"

            # Step 3: Create an issue
            printf "${BLUE}Step 3: Create an issue${NC}\n"
            printf "  \$ aspen-cli issue create --repo %s -t 'First issue' -b 'Testing issue tracking'\n" "${repo_id:0:16}..."
            local issue_output
            if issue_output=$(cli "$ticket" issue create --repo "$repo_id" -t "First issue" -b "Testing issue tracking" 2>&1); then
                printf "${GREEN}%s${NC}\n\n" "$issue_output"

                # Get full issue ID from JSON output
                local issue_json
                issue_json=$(cli "$ticket" --json issue list --repo "$repo_id" 2>&1 | grep -v WARN || echo "")
                local issue_id
                issue_id=$(echo "$issue_json" | grep -oE '"id": "[a-f0-9]{64}"' | head -1 | grep -oE '[a-f0-9]{64}' || echo "")

                if [ -n "$issue_id" ]; then
                    # Step 4: List issues
                    printf "${BLUE}Step 4: List issues${NC}\n"
                    printf "  \$ aspen-cli issue list --repo %s\n" "${repo_id:0:16}..."
                    cli "$ticket" issue list --repo "$repo_id" 2>&1 || true
                    printf "\n"

                    # Step 5: Add a comment
                    printf "${BLUE}Step 5: Add a comment${NC}\n"
                    printf "  \$ aspen-cli issue comment --repo %s %s -b 'This is a comment'\n" "${repo_id:0:16}..." "${issue_id:0:16}..."
                    cli "$ticket" issue comment --repo "$repo_id" "$issue_id" -b "This is a comment" 2>&1 || true
                    printf "\n"

                    # Step 6: Show issue details
                    printf "${BLUE}Step 6: Show issue details${NC}\n"
                    printf "  \$ aspen-cli issue show --repo %s %s\n" "${repo_id:0:16}..." "${issue_id:0:16}..."
                    cli "$ticket" issue show --repo "$repo_id" "$issue_id" 2>&1 || true
                    printf "\n"
                fi
            else
                printf "${YELLOW}Issue creation: %s${NC}\n\n" "$issue_output"
            fi

            # Step 7: List branches
            printf "${BLUE}Step 7: List branches${NC}\n"
            printf "  \$ aspen-cli branch list --repo %s\n" "${repo_id:0:16}..."
            cli "$ticket" branch list --repo "$repo_id" 2>&1 || true
            printf "\n"

        fi
    else
        printf "${YELLOW}%s${NC}\n\n" "$output"
    fi

    printf "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
    printf "${GREEN}Demo complete!${NC}\n\n"
    printf "Explore more with:\n"
    printf "  export ASPEN_TICKET='%s'\n" "$ticket"
    printf "  aspen-cli --help\n"
    printf "\n"
}

# Show help
cmd_help() {
    print_header

    printf "${YELLOW}Usage:${NC} %s [command]\n\n" "$0"

    printf "${YELLOW}Commands:${NC}\n"
    printf "  start   Start a Forge-enabled Aspen cluster (default)\n"
    printf "  stop    Stop the running cluster\n"
    printf "  demo    Run an interactive demo of Forge features\n"
    printf "  help    Show this help message\n"
    printf "\n"

    printf "${YELLOW}Environment Variables:${NC}\n"
    printf "  ASPEN_NODE_COUNT  Number of nodes (default: 1)\n"
    printf "  ASPEN_STORAGE     Storage backend: inmemory, redb (default: redb)\n"
    printf "  ASPEN_DATA_DIR    Data directory (default: /tmp/aspen-forge-PID)\n"
    printf "  ASPEN_LOG_LEVEL   Log level: trace, debug, info, warn (default: info)\n"
    printf "\n"

    printf "${YELLOW}Examples:${NC}\n"
    printf "  %s start                     # Start single-node Forge cluster\n" "$0"
    printf "  %s demo                      # Run interactive demo\n" "$0"
    printf "  ASPEN_NODE_COUNT=3 %s start  # Start 3-node cluster\n" "$0"
    printf "  %s stop                      # Stop the cluster\n" "$0"
    printf "\n"

    printf "${YELLOW}About Forge:${NC}\n"
    printf "  Forge is a decentralized code collaboration system built on Aspen.\n"
    printf "  It provides Git-like operations with BLAKE3 content-addressing,\n"
    printf "  Raft-consistent refs, and collaborative objects (issues, patches).\n"
    printf "\n"
    printf "  Architecture:\n"
    printf "  - Git objects stored in iroh-blobs (content-addressed)\n"
    printf "  - Refs (branches/tags) stored with Raft consensus\n"
    printf "  - Issues and patches as immutable DAGs (like Git commits)\n"
    printf "  - P2P sync via iroh-gossip announcements\n"
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
