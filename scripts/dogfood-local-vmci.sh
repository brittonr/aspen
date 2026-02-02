#!/usr/bin/env bash
# Aspen Self-Hosting with Local Nodes and VM-Isolated CI
#
# This script runs the Aspen dogfood workflow using:
# - Local processes for Aspen nodes (fast startup, easy debugging)
# - Cloud Hypervisor microVMs for CI jobs (hardware-level isolation)
#
# The nix builds run inside isolated VMs with:
# - virtiofs sharing of /nix/store (read-only)
# - Ephemeral workspace per job
# - Writable store overlay for build artifacts
#
# Usage:
#   nix run .#dogfood-local-vmci               # Launch default 1-node cluster
#   ASPEN_NODE_COUNT=3 nix run .#dogfood-local-vmci  # 3-node cluster
#   nix run .#dogfood-local-vmci -- help       # Show help
#
# Environment variables:
#   ASPEN_NODE_COUNT     - Number of nodes (default: 1, max: 10)
#   ASPEN_DATA_DIR       - Data directory (default: /tmp/aspen-dogfood-local-vmci)
#   ASPEN_LOG_LEVEL      - Log level (default: info)

set -eu

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="${PROJECT_DIR:-$(cd "$SCRIPT_DIR/.." && pwd)}"

# Source shared functions
. "$SCRIPT_DIR/lib/cluster-common.sh"

# Configuration
NODE_COUNT="${ASPEN_NODE_COUNT:-1}"
DATA_DIR="${ASPEN_DATA_DIR:-/tmp/aspen-dogfood-local-vmci}"
LOG_LEVEL="${ASPEN_LOG_LEVEL:-info}"
COOKIE="dogfood-vmci-$(date +%s)"

# Binaries (set by flake.nix wrapper)
ASPEN_NODE_BIN="${ASPEN_NODE_BIN:-$(find_binary aspen-node)}"
ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-$(find_binary aspen-cli)}"

# CI VM components (built dynamically)
CI_VM_KERNEL=""
CI_VM_INITRD=""
CI_VM_TOPLEVEL=""

# Binaries for CloudHypervisorWorker
CLOUD_HYPERVISOR_BIN="${CLOUD_HYPERVISOR_BIN:-$(find_binary cloud-hypervisor)}"
VIRTIOFSD_BIN="${VIRTIOFSD_BIN:-$(find_binary virtiofsd)}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
DIM='\033[2m'
NC='\033[0m'

# =============================================================================
# TIMING INFRASTRUCTURE
# =============================================================================

# Track timing data for summary
declare -A STEP_TIMES
declare -a STEP_ORDER
SCRIPT_START_TIME=""

# Get current timestamp in milliseconds
get_time_ms() {
    if command -v gdate >/dev/null 2>&1; then
        gdate +%s%3N
    elif date --version >/dev/null 2>&1; then
        date +%s%3N
    else
        # Fallback for systems without %N support (macOS)
        printf '%s000' "$(date +%s)"
    fi
}

# Format milliseconds as human-readable duration
format_duration() {
    local ms="$1"
    local seconds=$((ms / 1000))
    local remaining_ms=$((ms % 1000))

    if [ "$seconds" -ge 3600 ]; then
        local hours=$((seconds / 3600))
        local mins=$(((seconds % 3600) / 60))
        local secs=$((seconds % 60))
        printf "%dh %dm %ds" "$hours" "$mins" "$secs"
    elif [ "$seconds" -ge 60 ]; then
        local mins=$((seconds / 60))
        local secs=$((seconds % 60))
        printf "%dm %d.%03ds" "$mins" "$secs" "$remaining_ms"
    elif [ "$seconds" -ge 10 ]; then
        printf "%d.%01ds" "$seconds" "$((remaining_ms / 100))"
    else
        printf "%d.%03ds" "$seconds" "$remaining_ms"
    fi
}

# Start timing a step
timer_start() {
    local step_name="$1"
    eval "TIMER_${step_name}=$(get_time_ms)"
}

# End timing a step and record it
timer_end() {
    local step_name="$1"
    local end_time
    end_time=$(get_time_ms)

    local start_var="TIMER_${step_name}"
    local start_time="${!start_var:-$end_time}"

    local duration=$((end_time - start_time))
    STEP_TIMES["$step_name"]="$duration"
    STEP_ORDER+=("$step_name")
}

# Print timing for a step inline
print_step_time() {
    local step_name="$1"
    local duration="${STEP_TIMES[$step_name]:-0}"
    printf " ${DIM}[%s]${NC}" "$(format_duration "$duration")"
}

# Print timing summary
print_timing_summary() {
    local total_time="$1"

    printf "\n${BLUE}======================================${NC}\n"
    printf "${CYAN}Timing Summary${NC}\n"
    printf "${BLUE}======================================${NC}\n"

    # Calculate column widths
    local max_name_len=0
    for step in "${STEP_ORDER[@]}"; do
        local len=${#step}
        if [ "$len" -gt "$max_name_len" ]; then
            max_name_len=$len
        fi
    done

    # Print each step's timing
    for step in "${STEP_ORDER[@]}"; do
        local duration="${STEP_TIMES[$step]}"
        local formatted
        formatted=$(format_duration "$duration")
        local pct=$((duration * 100 / total_time))

        # Create a visual bar
        local bar_len=$((pct / 2))
        local bar=""
        for _ in $(seq 1 $bar_len); do bar="${bar}#"; done

        printf "  %-${max_name_len}s  %10s  %3d%%  ${GREEN}%s${NC}\n" \
            "$step" "$formatted" "$pct" "$bar"
    done

    printf "${BLUE}--------------------------------------${NC}\n"
    printf "  %-${max_name_len}s  %10s  100%%\n" "TOTAL" "$(format_duration "$total_time")"
    printf "${BLUE}======================================${NC}\n"

    # Save timing data to file
    local timing_file="$DATA_DIR/timing.json"
    {
        printf '{\n'
        printf '  "total_ms": %d,\n' "$total_time"
        printf '  "total_formatted": "%s",\n' "$(format_duration "$total_time")"
        printf '  "steps": {\n'
        local first=true
        for step in "${STEP_ORDER[@]}"; do
            if [ "$first" = "true" ]; then
                first=false
            else
                printf ',\n'
            fi
            printf '    "%s": %d' "$step" "${STEP_TIMES[$step]}"
        done
        printf '\n  }\n'
        printf '}\n'
    } > "$timing_file"
    printf "\n${DIM}Timing data saved to: %s${NC}\n" "$timing_file"
}

# Track node PIDs for cleanup
declare -a NODE_PIDS=()

# Cleanup function
cleanup() {
    printf "\n${BLUE}Cleaning up nodes...${NC}\n"

    for pid in "${NODE_PIDS[@]}"; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            printf "  Stopping node (PID %s)..." "$pid"
            kill "$pid" 2>/dev/null || true
            sleep 0.3
            if kill -0 "$pid" 2>/dev/null; then
                kill -9 "$pid" 2>/dev/null || true
            fi
            printf " ${GREEN}done${NC}\n"
        fi
    done

    # Clean up any orphaned cloud-hypervisor or virtiofsd processes
    pkill -f "cloud-hypervisor.*aspen-ci" 2>/dev/null || true
    pkill -f "virtiofsd.*aspen-ci" 2>/dev/null || true

    # Clean up CI VM sockets and workspaces
    rm -rf "$DATA_DIR/ci-workspaces" 2>/dev/null || true

    printf "${GREEN}Cleanup complete${NC}\n"
}

trap cleanup EXIT INT TERM

# =============================================================================
# NETWORK SETUP
# =============================================================================

# Network bridge for CI VMs
BRIDGE_NAME="aspen-ci-br0"
BRIDGE_IP="10.200.0.1/24"

# Set up network bridge, NAT, and TAP devices for CI VMs.
# All privileged operations are batched into a single sudo call to avoid
# multiple password prompts and sudo timeout issues.
setup_network() {
    printf "${BLUE}Setting up CI VM network...${NC}\n"

    # Generate list of TAP devices to create
    local tap_devices=""
    for node_id in $(seq 1 "$NODE_COUNT"); do
        for vm_idx in $(seq 0 7); do
            local tap_name="aspen-ci-n${node_id}-vm${vm_idx}-tap"
            # Only add if doesn't already exist
            if ! ip link show "$tap_name" &>/dev/null 2>&1; then
                tap_devices="$tap_devices $tap_name"
            fi
        done
    done

    # Check what needs to be done
    local need_sudo=false

    if ip link show "$BRIDGE_NAME" &>/dev/null 2>&1; then
        printf "  Bridge %s already exists\n" "$BRIDGE_NAME"
    else
        need_sudo=true
    fi

    if [ -n "$tap_devices" ]; then
        need_sudo=true
    fi

    # If nothing needs sudo, we're done
    if [ "$need_sudo" = "false" ]; then
        printf "  ${GREEN}Network already configured${NC}\n\n"
        return 0
    fi

    # Create a script that does all privileged operations in one go.
    # This keeps sudo credentials cached and provides atomic setup.
    local setup_script
    setup_script=$(mktemp)
    chmod +x "$setup_script"

    cat > "$setup_script" << 'SETUP_EOF'
#!/bin/sh
set -e
BRIDGE_NAME="$1"
BRIDGE_IP="$2"
TAP_USER="$3"
shift 3
TAP_DEVICES="$*"

# Create bridge if needed
if ! ip link show "$BRIDGE_NAME" >/dev/null 2>&1; then
    ip link add "$BRIDGE_NAME" type bridge
    ip addr add "$BRIDGE_IP" dev "$BRIDGE_NAME" 2>/dev/null || true
    ip link set "$BRIDGE_NAME" up
    echo "BRIDGE_CREATED"
fi

# Enable IP forwarding
sysctl -w net.ipv4.ip_forward=1 >/dev/null 2>&1 || true

# Set up NAT if not already configured
if ! iptables -t nat -C POSTROUTING -s 10.200.0.0/24 ! -o "$BRIDGE_NAME" -j MASQUERADE 2>/dev/null; then
    iptables -t nat -A POSTROUTING -s 10.200.0.0/24 ! -o "$BRIDGE_NAME" -j MASQUERADE 2>/dev/null || true
fi

# Create TAP devices
created=0
for tap_name in $TAP_DEVICES; do
    if ! ip link show "$tap_name" >/dev/null 2>&1; then
        if ip tuntap add "$tap_name" mode tap user "$TAP_USER"; then
            ip link set "$tap_name" master "$BRIDGE_NAME" 2>/dev/null || true
            ip link set "$tap_name" up 2>/dev/null || true
            created=$((created + 1))
        fi
    fi
done

echo "TAP_CREATED=$created"
SETUP_EOF

    # Run the setup script with sudo (single password prompt)
    printf "  Running privileged network setup..."
    local output
    if output=$(sudo "$setup_script" "$BRIDGE_NAME" "$BRIDGE_IP" "$USER" $tap_devices 2>&1); then
        printf " ${GREEN}done${NC}\n"

        # Parse output for details
        if echo "$output" | grep -q "BRIDGE_CREATED"; then
            printf "    Bridge %s created\n" "$BRIDGE_NAME"
        fi

        local tap_count
        tap_count=$(echo "$output" | grep "TAP_CREATED=" | cut -d= -f2)
        if [ -n "$tap_count" ] && [ "$tap_count" -gt 0 ]; then
            printf "    %s TAP devices created\n" "$tap_count"
        elif [ -z "$tap_devices" ] || [ "$tap_devices" = " " ]; then
            printf "    TAP devices already exist\n"
        fi
    else
        printf " ${YELLOW}failed${NC}\n"
        printf "  ${DIM}Error: %s${NC}\n" "$output"
        printf "  ${DIM}VMs may have no network access${NC}\n"
    fi

    rm -f "$setup_script"
    printf "\n"
}

# Build CI VM components
build_ci_vm_components() {
    printf "${BLUE}Building CI VM components...${NC}\n"

    printf "  Building kernel..."
    local kernel_output
    if ! kernel_output=$(nix build --no-link --print-out-paths .#ci-vm-kernel 2>/dev/null); then
        printf " ${RED}failed${NC}\n"
        return 1
    fi
    # Extract just the store path (last line with /nix/store)
    CI_VM_KERNEL=$(echo "$kernel_output" | grep '^/nix/store' | tail -1)
    printf " ${GREEN}done${NC}\n"

    printf "  Building initrd..."
    local initrd_output
    if ! initrd_output=$(nix build --no-link --print-out-paths .#ci-vm-initrd 2>/dev/null); then
        printf " ${RED}failed${NC}\n"
        return 1
    fi
    CI_VM_INITRD=$(echo "$initrd_output" | grep '^/nix/store' | tail -1)
    printf " ${GREEN}done${NC}\n"

    printf "  Building toplevel..."
    local toplevel_output
    if ! toplevel_output=$(nix build --no-link --print-out-paths .#ci-vm-toplevel 2>/dev/null); then
        printf " ${RED}failed${NC}\n"
        return 1
    fi
    CI_VM_TOPLEVEL=$(echo "$toplevel_output" | grep '^/nix/store' | tail -1)
    printf " ${GREEN}done${NC}\n"

    # Verify paths exist and find the actual kernel/initrd files
    if [ -f "$CI_VM_KERNEL/bzImage" ]; then
        CI_VM_KERNEL="$CI_VM_KERNEL/bzImage"
    elif [ -f "$CI_VM_KERNEL/Image" ]; then
        CI_VM_KERNEL="$CI_VM_KERNEL/Image"
    else
        printf "  ${RED}Kernel not found in %s${NC}\n" "$CI_VM_KERNEL"
        return 1
    fi

    if [ -f "$CI_VM_INITRD/initrd" ]; then
        CI_VM_INITRD="$CI_VM_INITRD/initrd"
    else
        printf "  ${RED}Initrd not found in %s${NC}\n" "$CI_VM_INITRD"
        return 1
    fi

    printf "  ${CYAN}Kernel:${NC}   %s\n" "$CI_VM_KERNEL"
    printf "  ${CYAN}Initrd:${NC}   %s\n" "$CI_VM_INITRD"
    printf "  ${CYAN}Toplevel:${NC} %s\n" "$CI_VM_TOPLEVEL"
    printf "\n"
}

# Check prerequisites
check_prereqs() {
    if [ -z "$ASPEN_NODE_BIN" ] || [ ! -x "$ASPEN_NODE_BIN" ]; then
        printf "${RED}Error: aspen-node not found.${NC}\n"
        printf "Run from flake: nix run .#dogfood-local-vmci\n"
        exit 1
    fi

    if [ -z "$ASPEN_CLI_BIN" ] || [ ! -x "$ASPEN_CLI_BIN" ]; then
        printf "${RED}Error: aspen-cli not found.${NC}\n"
        printf "Run from flake: nix run .#dogfood-local-vmci\n"
        exit 1
    fi

    if [ "$NODE_COUNT" -lt 1 ] || [ "$NODE_COUNT" -gt 10 ]; then
        printf "${RED}Error: ASPEN_NODE_COUNT must be 1-10 (got: %s)${NC}\n" "$NODE_COUNT"
        exit 1
    fi

    # Check for KVM (required for Cloud Hypervisor)
    if [ ! -e /dev/kvm ]; then
        printf "${RED}Error: KVM not available (/dev/kvm missing)${NC}\n"
        printf "Cloud Hypervisor VMs require hardware virtualization.\n"
        printf "On NixOS, enable: virtualisation.libvirtd.enable = true;\n"
        exit 1
    fi

    # Check for cloud-hypervisor
    if [ -z "$CLOUD_HYPERVISOR_BIN" ]; then
        printf "${RED}Error: cloud-hypervisor not found${NC}\n"
        printf "Run via: nix run .#dogfood-local-vmci\n"
        exit 1
    fi

    # Check for virtiofsd
    if [ -z "$VIRTIOFSD_BIN" ]; then
        printf "${RED}Error: virtiofsd not found${NC}\n"
        printf "Run via: nix run .#dogfood-local-vmci\n"
        exit 1
    fi
}

# Start a single node with CloudHypervisorWorker
start_node() {
    local node_id="$1"
    local node_data_dir="$DATA_DIR/node$node_id"
    local secret_key
    secret_key=$(generate_secret_key "$node_id")
    local log_file="$node_data_dir/node.log"
    local ci_workspace_dir="$DATA_DIR/ci-workspaces"

    mkdir -p "$node_data_dir"
    mkdir -p "$ci_workspace_dir"

    printf "  Starting node %d..." "$node_id"

    # Start node with CloudHypervisorWorker configuration
    RUST_LOG="$LOG_LEVEL" \
    ASPEN_BLOBS_ENABLED="true" \
    ASPEN_CI_ENABLED="true" \
    ASPEN_CI_KERNEL_PATH="$CI_VM_KERNEL" \
    ASPEN_CI_INITRD_PATH="$CI_VM_INITRD" \
    ASPEN_CI_TOPLEVEL_PATH="$CI_VM_TOPLEVEL" \
    CLOUD_HYPERVISOR_PATH="$CLOUD_HYPERVISOR_BIN" \
    VIRTIOFSD_PATH="$VIRTIOFSD_BIN" \
    ASPEN_CI_WORKSPACE_DIR="$ci_workspace_dir" \
    ASPEN_CI_VM_POOL_SIZE="1" \
    ASPEN_FORGE_ENABLE_GOSSIP="true" \
    "$ASPEN_NODE_BIN" \
        --node-id "$node_id" \
        --cookie "$COOKIE" \
        --iroh-secret-key "$secret_key" \
        --data-dir "$node_data_dir" \
        --storage-backend inmemory \
        --enable-workers \
        --worker-count 1 \
        --enable-ci \
        > "$log_file" 2>&1 &

    local pid=$!
    NODE_PIDS+=("$pid")
    echo "$pid" > "$node_data_dir/node.pid"

    printf " ${GREEN}PID %s${NC}\n" "$pid"
}

# Get ticket from node 1
get_ticket() {
    local node_log="$DATA_DIR/node1/node.log"
    local timeout=30
    local elapsed=0

    while [ "$elapsed" -lt "$timeout" ]; do
        if [ -f "$node_log" ]; then
            local ticket
            ticket=$(grep -oE 'aspen[a-z2-7]{50,200}' "$node_log" 2>/dev/null | head -1 || true)
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

# Get endpoint ID from node log
get_endpoint_id() {
    local log_file="$1"
    sed 's/\x1b\[[0-9;]*m//g' "$log_file" 2>/dev/null | \
        grep -oE 'endpoint_id=[a-f0-9]{64}' | head -1 | cut -d= -f2 || true
}

# Initialize cluster
init_cluster() {
    local ticket="$1"

    printf "${BLUE}Initializing cluster...${NC}\n"

    # Initialize cluster (node 1 becomes leader)
    printf "  Initializing..."
    local attempts=0
    local max_attempts=10
    while [ "$attempts" -lt "$max_attempts" ]; do
        if "$ASPEN_CLI_BIN" --quiet --ticket "$ticket" cluster init >/dev/null 2>&1; then
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
                if "$ASPEN_CLI_BIN" --quiet --ticket "$ticket" cluster add-learner \
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
        if "$ASPEN_CLI_BIN" --quiet --ticket "$ticket" cluster change-membership $members >/dev/null 2>&1; then
            printf " ${GREEN}done${NC}\n"
        else
            printf " ${YELLOW}skipped (may already be voters)${NC}\n"
        fi
    fi

    # Wait for cluster to stabilize
    printf "  Waiting for cluster to stabilize..."
    if wait_for_cluster_stable "$ASPEN_CLI_BIN" "$ticket" 30000 60; then
        printf " ${GREEN}done${NC}\n"
    else
        printf " ${YELLOW}timeout (continuing)${NC}\n"
    fi

    return 0
}

# Initialize forge repo
init_repo() {
    local ticket="$1"

    printf "${BLUE}Setting up Forge repository...${NC}\n"

    # Wait for forge subsystem
    printf "  Waiting for forge..."
    if wait_for_subsystem "$ASPEN_CLI_BIN" "$ticket" 30000 forge 60; then
        printf " ${GREEN}ready${NC}\n"
    else
        printf " ${RED}timeout${NC}\n"
        return 1
    fi

    # Wait for job subsystem (needed for CI)
    printf "  Waiting for job service..."
    if wait_for_subsystem "$ASPEN_CLI_BIN" "$ticket" 30000 job 60; then
        printf " ${GREEN}ready${NC}\n"
    else
        printf " ${YELLOW}timeout (continuing)${NC}\n"
    fi

    # Create repo using git init
    printf "  Creating repository..."
    local output
    if ! output=$("$ASPEN_CLI_BIN" --quiet --ticket "$ticket" git init \
        --description "Aspen distributed systems platform (VM-isolated CI)" \
        "aspen" 2>&1); then
        if echo "$output" | grep -q "already exists"; then
            printf " ${YELLOW}already exists${NC}\n"
            output=$("$ASPEN_CLI_BIN" --quiet --ticket "$ticket" git list 2>&1)
        else
            printf " ${RED}failed${NC}\n"
            printf "  Error: %s\n" "$output"
            return 1
        fi
    else
        printf " ${GREEN}done${NC}\n"
    fi

    # Extract repo ID
    local repo_id
    repo_id=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || true)
    if [ -z "$repo_id" ]; then
        printf "  ${RED}Could not extract repo ID${NC}\n"
        return 1
    fi

    printf '%s' "$repo_id" > "$DATA_DIR/repo_id.txt"
    printf "  Repo ID: %s\n" "$repo_id"

    # Configure git remote
    local remote_url="aspen://$ticket/$repo_id"
    if git -C "$PROJECT_DIR" remote get-url aspen >/dev/null 2>&1; then
        git -C "$PROJECT_DIR" remote set-url aspen "$remote_url"
    else
        git -C "$PROJECT_DIR" remote add aspen "$remote_url"
    fi
    printf "  Git remote configured\n"

    # Enable CI watching
    printf "  Enabling CI auto-trigger..."
    if "$ASPEN_CLI_BIN" --quiet --ticket "$ticket" ci watch "$repo_id" >/dev/null 2>&1; then
        printf " ${GREEN}done${NC}\n"
        sleep 2
    else
        printf " ${YELLOW}skipped${NC}\n"
    fi

    return 0
}

# Push to Aspen
push_to_aspen() {
    local branch="${1:-main}"

    printf "${BLUE}Pushing to Aspen Forge...${NC}\n"

    local git_remote_bin
    git_remote_bin=$(find_binary git-remote-aspen)

    if [ -z "$git_remote_bin" ]; then
        printf "  ${RED}git-remote-aspen not found${NC}\n"
        return 1
    fi

    local bin_dir
    bin_dir=$(dirname "$git_remote_bin")

    printf "  Branch: %s\n" "$branch"
    PATH="$bin_dir:$PATH" git -C "$PROJECT_DIR" push aspen "$branch"

    printf "  ${GREEN}Push complete${NC}\n"
}

# Trigger CI run manually (since auto-trigger may not be registered)
trigger_ci() {
    local ticket="$1"
    local repo_id="$2"

    printf "${BLUE}Triggering CI pipeline...${NC}\n"

    local output
    if output=$("$ASPEN_CLI_BIN" --quiet --ticket "$ticket" ci run "$repo_id" 2>&1); then
        local run_id
        run_id=$(echo "$output" | grep -oE '[a-f0-9-]{36}' | head -1 || true)
        if [ -n "$run_id" ]; then
            printf "  ${GREEN}Pipeline triggered${NC}: %s\n" "$run_id"
            echo "$run_id" > "$DATA_DIR/run_id.txt"
            return 0
        fi
    fi

    printf "  ${RED}Failed to trigger CI${NC}\n"
    printf "  Output: %s\n" "$output"
    return 1
}

# Wait for CI to complete
wait_for_ci() {
    local ticket="$1"
    local timeout="${2:-600}"
    local elapsed=0

    printf "${BLUE}Waiting for CI to complete...${NC}\n"

    # Give CI a moment to trigger
    sleep 3

    while [ "$elapsed" -lt "$timeout" ]; do
        local output
        output=$("$ASPEN_CLI_BIN" --quiet --ticket "$ticket" ci list --limit 1 2>&1 || true)

        # Case-insensitive check for success/completed
        if echo "$output" | grep -iq "success\|completed"; then
            printf "  ${GREEN}CI completed successfully${NC}\n"
            "$ASPEN_CLI_BIN" --ticket "$ticket" ci list --limit 1
            return 0
        elif echo "$output" | grep -iq "failed\|failure"; then
            printf "  ${RED}CI failed${NC}\n"
            "$ASPEN_CLI_BIN" --ticket "$ticket" ci list --limit 1
            return 1
        elif echo "$output" | grep -iq "running\|pending\|in.progress"; then
            printf "  CI in progress... (%ds)\r" "$elapsed"
        fi

        sleep 5
        elapsed=$((elapsed + 5))
    done

    printf "\n  ${YELLOW}CI did not complete within %ds${NC}\n" "$timeout"
    return 1
}

# Print cluster info
print_info() {
    local ticket="$1"

    printf "\n${BLUE}======================================${NC}\n"
    printf "${GREEN}Aspen Local Cluster Ready${NC} (%d nodes)\n" "$NODE_COUNT"
    printf "${BLUE}======================================${NC}\n"
    printf "\n"
    printf "Cookie:     %s\n" "$COOKIE"
    printf "Data dir:   %s\n" "$DATA_DIR"
    printf "Ticket:     %s/ticket.txt\n" "$DATA_DIR"
    printf "\n"

    printf "${BLUE}CI Configuration:${NC}\n"
    printf "  Worker type:  CloudHypervisorWorker (VM isolation)\n"
    printf "  VM pool size: 1\n"
    printf "  Kernel:       %s\n" "$CI_VM_KERNEL"
    printf "  Initrd:       %s\n" "$CI_VM_INITRD"
    printf "  Toplevel:     %s\n" "$CI_VM_TOPLEVEL"
    printf "\n"

    printf "${BLUE}CLI Commands:${NC}\n"
    printf "  aspen-cli --ticket %s cluster status\n" "$ticket"
    printf "  aspen-cli --ticket %s ci status\n" "$ticket"
    printf "  aspen-cli --ticket %s ci list\n" "$ticket"
    printf "\n"

    printf "${BLUE}Logs:${NC}\n"
    for node_id in $(seq 1 "$NODE_COUNT"); do
        printf "  Node %d: %s/node%d/node.log\n" "$node_id" "$DATA_DIR" "$node_id"
    done
    printf "\n"

    printf "${BLUE}Stop cluster:${NC}\n"
    printf "  Press Ctrl+C\n"
    printf "${BLUE}======================================${NC}\n"
}

# Run complete workflow
cmd_run() {
    local branch="${1:-main}"

    # Start overall timer
    SCRIPT_START_TIME=$(get_time_ms)

    printf "${BLUE}============================================${NC}\n"
    printf "${GREEN}Aspen Self-Hosting (Local + VM CI)${NC}\n"
    printf "${BLUE}============================================${NC}\n\n"

    check_prereqs
    rm -rf "$DATA_DIR"
    mkdir -p "$DATA_DIR"

    # Step 0a: Set up network bridge for CI VMs
    timer_start "network_setup"
    setup_network
    timer_end "network_setup"

    # Step 0b: Build CI VM components
    timer_start "ci_vm_build"
    if ! build_ci_vm_components; then
        printf "${RED}Failed to build CI VM components${NC}\n"
        exit 1
    fi
    timer_end "ci_vm_build"
    print_step_time "ci_vm_build"
    printf "\n"

    # Step 1: Start nodes
    timer_start "nodes_start"
    printf "${BLUE}Starting nodes...${NC}\n"
    for node_id in $(seq 1 "$NODE_COUNT"); do
        start_node "$node_id"
    done
    printf "\n"

    # Step 2: Wait for nodes to be ready
    printf "${BLUE}Waiting for nodes to start...${NC}\n"
    sleep 3
    timer_end "nodes_start"
    print_step_time "nodes_start"
    printf "\n\n"

    # Step 3: Get ticket
    timer_start "get_ticket"
    printf "  Getting ticket..."
    local ticket
    if ! ticket=$(get_ticket); then
        printf " ${RED}failed${NC}\n"
        printf "Check logs: %s/node1/node.log\n" "$DATA_DIR"
        exit 1
    fi
    printf " ${GREEN}done${NC}\n"
    echo "$ticket" > "$DATA_DIR/ticket.txt"
    timer_end "get_ticket"
    print_step_time "get_ticket"
    printf "\n\n"

    # Step 4: Initialize cluster
    timer_start "cluster_init"
    if ! init_cluster "$ticket"; then
        printf "${RED}Cluster initialization failed${NC}\n"
        exit 1
    fi
    timer_end "cluster_init"
    print_step_time "cluster_init"
    printf "\n\n"

    # Step 5: Create repo and push
    timer_start "repo_setup"
    if ! init_repo "$ticket"; then
        printf "${YELLOW}Repository setup failed (may already exist)${NC}\n"
    fi
    timer_end "repo_setup"
    print_step_time "repo_setup"
    printf "\n\n"

    timer_start "git_push"
    push_to_aspen "$branch"
    timer_end "git_push"
    print_step_time "git_push"
    printf "\n\n"

    # Step 6: Trigger CI manually (auto-trigger may not be registered)
    local repo_id
    repo_id=$(cat "$DATA_DIR/repo_id.txt" 2>/dev/null || true)
    if [ -n "$repo_id" ]; then
        timer_start "ci_trigger"
        trigger_ci "$ticket" "$repo_id" || true
        timer_end "ci_trigger"
        print_step_time "ci_trigger"
        printf "\n\n"

        # Step 7: Wait for CI (30 min timeout for VM builds)
        timer_start "ci_wait"
        wait_for_ci "$ticket" 1800 || true
        timer_end "ci_wait"
        print_step_time "ci_wait"
        printf "\n"
    else
        printf "${YELLOW}No repo ID found, skipping CI${NC}\n"
    fi

    # Calculate total time and print summary
    local total_time
    total_time=$(($(get_time_ms) - SCRIPT_START_TIME))
    print_timing_summary "$total_time"

    # Print info and wait
    print_info "$ticket"

    printf "\n${YELLOW}Running in foreground. Press Ctrl+C to stop.${NC}\n"

    # Monitor nodes
    while true; do
        local all_running=true
        for pid in "${NODE_PIDS[@]}"; do
            if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
                all_running=false
                break
            fi
        done

        if [ "$all_running" = "false" ]; then
            printf "${RED}A node died unexpectedly${NC}\n"
            exit 1
        fi

        sleep 5
    done
}

# Show help
cmd_help() {
    printf "Aspen Self-Hosting with Local Nodes and VM-Isolated CI\n"
    printf "\n"
    printf "This script runs Aspen nodes as local processes, but executes\n"
    printf "CI jobs inside Cloud Hypervisor microVMs for hardware-level isolation.\n"
    printf "\n"
    printf "Usage: nix run .#dogfood-local-vmci [-- command]\n"
    printf "\n"
    printf "Commands:\n"
    printf "  run [branch]  - ${GREEN}Full workflow${NC} (build VMs, start nodes, init cluster, push, CI)\n"
    printf "  help          - Show this help message\n"
    printf "\n"
    printf "Environment variables:\n"
    printf "  ASPEN_NODE_COUNT  - Number of nodes (default: 1, max: 10)\n"
    printf "  ASPEN_DATA_DIR    - Data directory (default: /tmp/aspen-dogfood-local-vmci)\n"
    printf "  ASPEN_LOG_LEVEL   - Log level (default: info)\n"
    printf "\n"
    printf "Requirements:\n"
    printf "  - KVM enabled (/dev/kvm accessible)\n"
    printf "  - cloud-hypervisor and virtiofsd in PATH (provided by nix run)\n"
    printf "\n"
    printf "Example:\n"
    printf "  nix run .#dogfood-local-vmci                     # Single node\n"
    printf "  ASPEN_NODE_COUNT=3 nix run .#dogfood-local-vmci  # 3-node cluster\n"
}

# Main entry point
case "${1:-run}" in
    run)
        shift || true
        cmd_run "$@"
        ;;
    help|--help|-h)
        cmd_help
        ;;
    *)
        printf "${RED}Unknown command: %s${NC}\n" "$1"
        cmd_help
        exit 1
        ;;
esac
