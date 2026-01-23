#!/usr/bin/env bash
# Aspen Self-Hosting (Dogfooding) Script
#
# This script sets up and runs Aspen to host itself:
#   - Starts a 3-node cluster with Forge + CI + Nix Cache enabled
#   - Creates the Aspen repository in Forge
#   - Automatically configures git remote and CI watching
#
# Usage:
#   ./scripts/dogfood.sh run    # One command to do everything
#   ./scripts/dogfood.sh [start|stop|status|init-repo|push|help]
#
# Environment variables:
#   ASPEN_DOGFOOD_DIR    - Data directory (default: /tmp/aspen-dogfood)
#   ASPEN_BUILD_RELEASE  - Build in release mode: true/false (default: false)
#   ASPEN_NODE_COUNT     - Number of nodes (default: 3)
#   ASPEN_FOREGROUND     - Run in foreground (default: true)
#
# Examples:
#   ./scripts/dogfood.sh start        # Start dogfood cluster
#   ./scripts/dogfood.sh init-repo    # Create Aspen repo and configure git remote
#   ./scripts/dogfood.sh push         # Push to Aspen (handles PATH automatically)
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
# git-bridge is required for git-remote-aspen helper
DOGFOOD_FEATURES="ci,forge,git-bridge,nix-cache-gateway,shell-worker,blob"

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

    # Build with all required features (includes git-remote-aspen for git integration)
    # Note: aspen-cli is in a separate crate, so we need --workspace
    if ! cargo build $build_mode --features "$DOGFOOD_FEATURES" --workspace --bin aspen-node --bin aspen-cli --bin git-remote-aspen; then
        printf "${RED}Build failed${NC}\n"
        exit 1
    fi

    # Set binary paths
    ASPEN_NODE_BIN="$PROJECT_DIR/target/$target_dir/aspen-node"
    ASPEN_CLI_BIN="$PROJECT_DIR/target/$target_dir/aspen-cli"
    GIT_REMOTE_ASPEN_BIN="$PROJECT_DIR/target/$target_dir/git-remote-aspen"
    export ASPEN_NODE_BIN ASPEN_CLI_BIN GIT_REMOTE_ASPEN_BIN

    printf "  aspen-node:       %s\n" "$ASPEN_NODE_BIN"
    printf "  aspen-cli:        %s\n" "$ASPEN_CLI_BIN"
    printf "  git-remote-aspen: %s\n" "$GIT_REMOTE_ASPEN_BIN"
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

    # Check for saved repo ID to configure CI watching
    local ci_watched_repos="${ASPEN_CI_WATCHED_REPOS:-}"
    if [ -z "$ci_watched_repos" ] && [ -f "$DOGFOOD_DIR/repo_id.txt" ]; then
        ci_watched_repos=$(cat "$DOGFOOD_DIR/repo_id.txt")
    fi

    # Start node inside nix develop to ensure shell workers have access to nix/cargo tools.
    # This is critical because the ShellCommandWorker captures the parent process's PATH
    # at startup, so running the node inside nix develop ensures all nix store paths
    # are available for shell job execution.
    #
    # Uses CLI flags for most configuration, but ASPEN_CI_WATCHED_REPOS env var for watched repos.
    # Note: gossip and mdns are enabled by default.
    nix develop "$PROJECT_DIR" --command sh -c "
        RUST_LOG='${ASPEN_LOG_LEVEL:-info}' \
        ASPEN_CI_WATCHED_REPOS='$ci_watched_repos' \
        '$ASPEN_NODE_BIN' \
            --node-id '$node_id' \
            --cookie '$COOKIE' \
            --data-dir '$node_data_dir' \
            --storage-backend redb \
            --iroh-secret-key '$secret_key' \
            --enable-workers \
            --worker-count 2 \
            --enable-ci \
            --ci-auto-trigger
    " > "$log_file" 2>&1 &

    local pid=$!
    printf '%s\n' "$pid" >> "$PID_FILE"
    printf "  Node %d: PID %d (log: %s)\n" "$node_id" "$pid" "$log_file"
}

# Wait for ticket from node 1 log
# Returns ticket via stdout, status messages go to stderr
wait_for_ticket() {
    local log_file="$DOGFOOD_DIR/node1/node.log"
    local timeout=30
    local elapsed=0

    printf "  Waiting for node 1 to start" >&2

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$log_file" ]; then
            local ticket
            ticket=$(grep -oE 'aspen[a-z2-7]{50,200}' "$log_file" 2>/dev/null | head -1 || true)

            if [ -n "$ticket" ]; then
                printf " ${GREEN}done${NC}\n" >&2
                printf '%s' "$ticket" > "$DOGFOOD_DIR/ticket.txt"
                printf '%s' "$ticket"
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
    printf "  1. Initialize repository and configure git remote:\n"
    printf "     %s init-repo\n" "$0"
    printf "\n"
    printf "  2. Push to trigger CI:\n"
    printf "     %s push\n" "$0"
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
    printf "Repository ID: %s\n\n" "$repo_id"

    # Automatically configure git remote
    local remote_url="aspen://$ticket/$repo_id"
    printf "${BLUE}Configuring git remote...${NC}\n"

    if git remote get-url aspen >/dev/null 2>&1; then
        printf "  Updating existing 'aspen' remote\n"
        git remote set-url aspen "$remote_url"
    else
        printf "  Adding new 'aspen' remote\n"
        git remote add aspen "$remote_url"
    fi
    printf "  Remote URL: %s\n" "$remote_url"
    printf "${GREEN}Git remote configured${NC}\n\n"

    # Watch the repository for CI auto-trigger
    printf "${BLUE}Enabling CI auto-trigger for repository...${NC}\n"
    if "$ASPEN_CLI_BIN" --ticket "$ticket" ci watch "$repo_id" >/dev/null 2>&1; then
        printf "  ${GREEN}CI watching enabled${NC}\n"
        sleep 2  # Allow async propagation of gossip subscription
    else
        printf "  ${YELLOW}CI watching not available - manual trigger required${NC}\n\n"
    fi

    printf "${BLUE}Ready to push! Run:${NC}\n"
    printf "  %s push\n" "$0"
    printf "\n"
}

# Push to Aspen Forge (handles PATH automatically)
cmd_push() {
    local branch="${1:-main}"

    if [ ! -f "$DOGFOOD_DIR/ticket.txt" ]; then
        printf "${RED}Error: No cluster running. Start with: %s start${NC}\n" "$0"
        exit 1
    fi

    if ! git remote get-url aspen >/dev/null 2>&1; then
        printf "${RED}Error: No 'aspen' remote configured. Run: %s init-repo${NC}\n" "$0"
        exit 1
    fi

    # Find git-remote-aspen binary
    GIT_REMOTE_ASPEN_BIN="${GIT_REMOTE_ASPEN_BIN:-$(find_binary git-remote-aspen)}"
    if [ -z "$GIT_REMOTE_ASPEN_BIN" ] || [ ! -x "$GIT_REMOTE_ASPEN_BIN" ]; then
        printf "${RED}Error: git-remote-aspen binary not found${NC}\n"
        printf "Build with: cargo build --bin git-remote-aspen --features git-bridge\n"
        exit 1
    fi

    # Get the directory containing git-remote-aspen
    local bin_dir
    bin_dir=$(dirname "$GIT_REMOTE_ASPEN_BIN")

    printf "${BLUE}Pushing to Aspen Forge...${NC}\n"
    printf "  Branch: %s\n" "$branch"
    printf "  Remote: %s\n" "$(git remote get-url aspen)"
    printf "\n"

    # Run git push with PATH set to find git-remote-aspen
    PATH="$bin_dir:$PATH" git push aspen "$branch"

    printf "\n${GREEN}Push complete!${NC}\n"

    # Show CI status hint
    if [ -f "$DOGFOOD_DIR/repo_id.txt" ]; then
        printf "\n${BLUE}Check CI status:${NC}\n"
        ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"
        local ticket
        ticket=$(cat "$DOGFOOD_DIR/ticket.txt")
        printf "  %s --ticket %s ci status\n" "$ASPEN_CLI_BIN" "$ticket"
    fi
}

# Verify CI build artifacts exist
cmd_verify() {
    if [ ! -f "$DOGFOOD_DIR/ticket.txt" ]; then
        printf "${RED}Error: No cluster running. Start with: %s start${NC}\n" "$0"
        exit 1
    fi

    ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"
    local ticket
    ticket=$(cat "$DOGFOOD_DIR/ticket.txt")

    printf "${BLUE}Verifying CI build artifacts...${NC}\n\n"

    # Check build artifacts directory
    printf "  Build artifacts directory: "
    local builds_dir="/tmp/aspen-ci/builds"
    if [ -d "$builds_dir" ]; then
        local count
        count=$(find "$builds_dir" -type f 2>/dev/null | wc -l)
        if [ "$count" -gt 0 ]; then
            printf "${GREEN}%d files${NC}\n" "$count"
            find "$builds_dir" -type f 2>/dev/null | head -5 | while read -r f; do
                printf "    %s\n" "$f"
            done
            if [ "$count" -gt 5 ]; then
                printf "    ... and %d more\n" "$((count - 5))"
            fi
        else
            printf "${YELLOW}empty${NC}\n"
        fi
    else
        printf "${YELLOW}not found${NC}\n"
    fi

    # Check blob store
    printf "  Blob store: "
    local blob_output
    if blob_output=$("$ASPEN_CLI_BIN" --ticket "$ticket" blob list --limit 10 2>&1); then
        local blob_count
        # Use grep -c and ensure single integer output
        blob_count=$(echo "$blob_output" | grep -c "Hash:" 2>/dev/null | tr -d '[:space:]')
        blob_count="${blob_count:-0}"
        if [ "$blob_count" -gt 0 ] 2>/dev/null; then
            printf "${GREEN}%d blobs${NC}\n" "$blob_count"
        else
            printf "${YELLOW}no blobs found${NC}\n"
        fi
    else
        printf "${YELLOW}unable to query${NC}\n"
    fi

    # Check cache stats
    printf "  Nix cache: "
    local cache_output
    if cache_output=$("$ASPEN_CLI_BIN" --ticket "$ticket" cache stats 2>&1); then
        printf "${GREEN}available${NC}\n"
        echo "$cache_output" | head -5 | sed 's/^/    /'
    else
        printf "${YELLOW}not available${NC}\n"
    fi

    printf "\n${GREEN}Verification complete${NC}\n"
}

# Run everything: start cluster, init repo, push
cmd_run() {
    local branch="${1:-main}"

    print_header
    printf "Running complete dogfood workflow\n\n"

    # Step 1: Ensure cluster is running
    if [ -f "$PID_FILE" ] && cmd_status >/dev/null 2>&1; then
        printf "${GREEN}Cluster already running${NC}\n\n"
    else
        printf "${BLUE}Step 1: Starting cluster...${NC}\n"
        # Run start in background mode
        ASPEN_FOREGROUND=false cmd_start_internal
    fi

    # Step 2: Ensure repo exists and git remote is configured
    if [ -f "$DOGFOOD_DIR/repo_id.txt" ] && git remote get-url aspen >/dev/null 2>&1; then
        printf "${GREEN}Repository already configured${NC}\n"
        printf "  Repo ID: %s\n" "$(cat "$DOGFOOD_DIR/repo_id.txt")"
        printf "  Remote:  %s\n" "$(git remote get-url aspen)"

        # Ensure CI is watching the repo (watch state may have been lost on restart)
        ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"
        local ticket
        ticket=$(cat "$DOGFOOD_DIR/ticket.txt")
        local repo_id
        repo_id=$(cat "$DOGFOOD_DIR/repo_id.txt")
        printf "  Ensuring CI watch is active..."
        if "$ASPEN_CLI_BIN" --ticket "$ticket" ci watch "$repo_id" >/dev/null 2>&1; then
            printf " ${GREEN}done${NC}\n"
            sleep 2  # Allow async propagation of gossip subscription
        else
            printf " ${YELLOW}skipped${NC}\n\n"
        fi
    else
        printf "${BLUE}Step 2: Initializing repository...${NC}\n"
        cmd_init_repo_internal

        # Set up CI watch for the newly created repo
        ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"
        local ticket
        ticket=$(cat "$DOGFOOD_DIR/ticket.txt")
        local repo_id
        repo_id=$(cat "$DOGFOOD_DIR/repo_id.txt")
        printf "  Setting up CI watch..."
        if "$ASPEN_CLI_BIN" --ticket "$ticket" ci watch "$repo_id" >/dev/null 2>&1; then
            printf " ${GREEN}done${NC}\n"
            sleep 2  # Allow async propagation of gossip subscription
        else
            printf " ${YELLOW}skipped${NC}\n\n"
        fi
    fi

    # Step 3: Push
    printf "${BLUE}Step 3: Pushing to Aspen Forge...${NC}\n"
    cmd_push "$branch"

    # Step 4: Wait for CI to start and monitor
    printf "\n${BLUE}Step 4: Waiting for CI pipeline...${NC}\n"
    ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"
    local ticket
    ticket=$(cat "$DOGFOOD_DIR/ticket.txt")

    # Poll for CI run with exponential backoff instead of fixed sleep.
    # This handles the async nature of:
    # 1. Trigger processing via channel
    # 2. Pipeline run persistence to KV store
    # 3. Raft replication latency
    local run_id=""
    local max_retries=30
    local retry_count=0
    local wait_time=1

    printf "  Waiting for pipeline to start"
    while [ "$retry_count" -lt "$max_retries" ] && [ -z "$run_id" ]; do
        run_id=$("$ASPEN_CLI_BIN" --ticket "$ticket" ci list --limit 1 2>/dev/null | grep -oE '[a-f0-9-]{36}' | head -1 || true)
        if [ -z "$run_id" ]; then
            printf "."
            sleep "$wait_time"
            retry_count=$((retry_count + 1))
            # Exponential backoff: 1, 2, 4, 4, 4... (cap at 4s)
            if [ "$wait_time" -lt 4 ]; then
                wait_time=$((wait_time * 2))
                [ "$wait_time" -gt 4 ] && wait_time=4
            fi
        fi
    done

    if [ -n "$run_id" ]; then
        printf " ${GREEN}found${NC}\n"
        printf "  Run ID: %s\n" "$run_id"

        # Poll for completion (max 10 minutes)
        local elapsed=0
        local max_wait=600
        while [ "$elapsed" -lt "$max_wait" ]; do
            local status_output
            status_output=$("$ASPEN_CLI_BIN" --ticket "$ticket" ci status "$run_id" 2>&1 || true)
            local status
            status=$(echo "$status_output" | grep -oE 'Status: [a-z]+' | head -1 | cut -d' ' -f2 || true)

            case "$status" in
                success)
                    printf "\n  ${GREEN}CI completed successfully${NC}\n"
                    break
                    ;;
                failed)
                    printf "\n  ${RED}CI failed${NC}\n"
                    break
                    ;;
                cancelled)
                    printf "\n  ${YELLOW}CI was cancelled${NC}\n"
                    break
                    ;;
                *)
                    printf "."
                    sleep 10
                    elapsed=$((elapsed + 10))
                    ;;
            esac
        done

        if [ "$elapsed" -ge "$max_wait" ]; then
            printf "\n  ${YELLOW}Timed out waiting for CI (still running)${NC}\n"
        fi
    else
        printf " ${YELLOW}timeout${NC}\n"
        printf "  ${YELLOW}No CI run detected after ${max_retries} attempts${NC}\n"
        printf "  This may indicate CI trigger didn't fire or replication is slow.\n"
        printf "  Check node logs: tail -f %s/node1/node.log\n" "$DOGFOOD_DIR"
        printf "  Manual trigger: %s --ticket %s ci run <repo_id>\n" "$ASPEN_CLI_BIN" "$ticket"
    fi

    # Step 5: Verify artifacts
    printf "\n${BLUE}Step 5: Verifying build artifacts...${NC}\n"
    cmd_verify
}

# Internal start (no foreground monitoring)
cmd_start_internal() {
    # Build if binaries don't exist
    ASPEN_NODE_BIN="${ASPEN_NODE_BIN:-$(find_binary aspen-node)}"
    ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"
    GIT_REMOTE_ASPEN_BIN="${GIT_REMOTE_ASPEN_BIN:-$(find_binary git-remote-aspen)}"

    if [ -z "$ASPEN_NODE_BIN" ] || [ -z "$ASPEN_CLI_BIN" ] || [ -z "$GIT_REMOTE_ASPEN_BIN" ]; then
        build_binaries
    else
        printf "Using existing binaries\n"
    fi

    # Clean up existing data but preserve repo_id if it exists
    local saved_repo_id=""
    if [ -f "$DOGFOOD_DIR/repo_id.txt" ]; then
        saved_repo_id=$(cat "$DOGFOOD_DIR/repo_id.txt")
    fi

    if [ -d "$DOGFOOD_DIR" ]; then
        cmd_stop 2>/dev/null || true
        rm -rf "$DOGFOOD_DIR"
    fi

    mkdir -p "$DOGFOOD_DIR"
    : > "$PID_FILE"

    # Restore repo_id if we had one
    if [ -n "$saved_repo_id" ]; then
        printf '%s' "$saved_repo_id" > "$DOGFOOD_DIR/repo_id.txt"
    fi

    # Start nodes
    printf "Starting %d nodes..." "$NODE_COUNT"
    local id=1
    while [ "$id" -le "$NODE_COUNT" ]; do
        start_node "$id" >/dev/null
        id=$((id + 1))
    done
    printf " ${GREEN}done${NC}\n"

    # Wait for ticket and init cluster
    printf "Forming cluster..."
    local ticket
    if ! ticket=$(wait_for_ticket 2>/dev/null); then
        printf " ${RED}failed${NC}\n"
        cmd_stop
        exit 1
    fi

    sleep 5  # gossip discovery
    "$ASPEN_CLI_BIN" --ticket "$ticket" cluster init >/dev/null 2>&1 || true

    # Add learners and promote
    if [ "$NODE_COUNT" -gt 1 ]; then
        local id=2
        while [ "$id" -le "$NODE_COUNT" ]; do
            local endpoint_id
            endpoint_id=$(get_endpoint_id "$DOGFOOD_DIR/node$id/node.log")
            if [ -n "$endpoint_id" ]; then
                "$ASPEN_CLI_BIN" --ticket "$ticket" cluster add-learner \
                    --node-id "$id" --addr "$endpoint_id" >/dev/null 2>&1 || true
            fi
            id=$((id + 1))
        done
        sleep 2

        local members=""
        id=1
        while [ "$id" -le "$NODE_COUNT" ]; do
            members="${members:+$members }$id"
            id=$((id + 1))
        done
        "$ASPEN_CLI_BIN" --ticket "$ticket" cluster change-membership $members >/dev/null 2>&1 || true
    fi

    printf " ${GREEN}done${NC}\n\n"
}

# Internal init-repo (quieter output)
cmd_init_repo_internal() {
    ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"

    local ticket
    ticket=$(cat "$DOGFOOD_DIR/ticket.txt")

    # Create the repository
    local output
    if ! output=$("$ASPEN_CLI_BIN" --ticket "$ticket" git init \
        --description "Aspen distributed systems platform (self-hosted)" \
        "aspen" 2>&1); then
        printf "${RED}Failed to create repository: %s${NC}\n" "$output"
        exit 1
    fi

    local repo_id
    repo_id=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || true)

    if [ -z "$repo_id" ]; then
        printf "${YELLOW}Repository may already exist${NC}\n"
        exit 1
    fi

    printf '%s' "$repo_id" > "$DOGFOOD_DIR/repo_id.txt"
    printf "  Repo ID: %s\n" "$repo_id"

    # Configure git remote
    local remote_url="aspen://$ticket/$repo_id"
    if git remote get-url aspen >/dev/null 2>&1; then
        git remote set-url aspen "$remote_url"
    else
        git remote add aspen "$remote_url"
    fi
    printf "  Remote configured\n"
}

# Start the cluster
cmd_start() {
    print_header
    printf "Starting %d-node dogfood cluster\n\n" "$NODE_COUNT"

    # Build if binaries don't exist
    ASPEN_NODE_BIN="${ASPEN_NODE_BIN:-$(find_binary aspen-node)}"
    ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"
    GIT_REMOTE_ASPEN_BIN="${GIT_REMOTE_ASPEN_BIN:-$(find_binary git-remote-aspen)}"

    if [ -z "$ASPEN_NODE_BIN" ] || [ -z "$ASPEN_CLI_BIN" ] || [ -z "$GIT_REMOTE_ASPEN_BIN" ]; then
        build_binaries
    else
        printf "Using existing binaries:\n"
        printf "  aspen-node:       %s\n" "$ASPEN_NODE_BIN"
        printf "  aspen-cli:        %s\n" "$ASPEN_CLI_BIN"
        printf "  git-remote-aspen: %s\n" "$GIT_REMOTE_ASPEN_BIN"
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
    printf "Usage: %s [command]\n" "$0"
    printf "\n"
    printf "Commands:\n"
    printf "  run        - ${GREEN}One command to do everything${NC} (start + init + push)\n"
    printf "  start      - Build and start the dogfood cluster\n"
    printf "  stop       - Stop all nodes\n"
    printf "  status     - Show cluster status\n"
    printf "  init-repo  - Create Aspen repo in Forge and configure git remote\n"
    printf "  push       - Push to Aspen Forge (handles PATH automatically)\n"
    printf "  verify     - Verify CI build artifacts exist in blob store\n"
    printf "  help       - Show this help message\n"
    printf "\n"
    printf "Quick start:\n"
    printf "  ${GREEN}%s run${NC}   # Does everything in one command\n" "$0"
    printf "\n"
    printf "Environment variables:\n"
    printf "  ASPEN_DOGFOOD_DIR    - Data directory (default: /tmp/aspen-dogfood)\n"
    printf "  ASPEN_BUILD_RELEASE  - Build release: true/false (default: false)\n"
    printf "  ASPEN_NODE_COUNT     - Number of nodes (default: 3)\n"
    printf "  ASPEN_LOG_LEVEL      - Log level (default: info)\n"
    printf "\n"
}

# Main
main() {
    local cmd="${1:-help}"
    shift || true

    case "$cmd" in
        run)
            cmd_run "$@"
            ;;
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
        push)
            cmd_push "$@"
            ;;
        verify)
            cmd_verify
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
