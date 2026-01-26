#!/usr/bin/env bash
# Aspen Self-Hosting in Cloud Hypervisor MicroVMs
#
# This script runs the Aspen dogfood workflow inside isolated Cloud Hypervisor
# microVMs for full kernel-level isolation, preventing CI jobs from affecting
# the host system.
#
# Usage:
#   nix run .#dogfood-vm               # Launch default 3-node cluster
#   ASPEN_NODE_COUNT=5 nix run .#dogfood-vm  # Launch 5-node cluster
#   nix run .#dogfood-vm -- run         # One command to do everything
#   nix run .#dogfood-vm -- help        # Show help
#
# Environment variables:
#   ASPEN_NODE_COUNT     - Number of VMs (default: 3, max: 10)
#   ASPEN_VM_DIR         - VM state directory (default: .aspen/vms)
#   ASPEN_LOG_LEVEL      - Log level (default: info)
#
# Requirements:
#   - KVM enabled (/dev/kvm accessible)
#   - First run: sudo for network bridge and TAP device setup
#   - Subsequent runs: no sudo needed (reuses existing network devices)
#   - microvm.nix flake input

set -eu

# Resolve script directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# Allow PROJECT_DIR override from environment (needed when scripts are in Nix store)
PROJECT_DIR="${PROJECT_DIR:-$(cd "$SCRIPT_DIR/.." && pwd)}"

# Source shared functions
. "$SCRIPT_DIR/lib/cluster-common.sh"
. "$SCRIPT_DIR/lib/network-diagnostics.sh"

# Configuration (set by flake.nix wrapper, but provide defaults)
NODE_COUNT="${ASPEN_NODE_COUNT:-3}"
VM_DIR="${ASPEN_VM_DIR:-$PROJECT_DIR/.aspen/vms}"
COOKIE="dogfood-vm-$(date +%s)"

# Per-stage boot timeouts (configurable via environment)
STAGE_PROCESS_TIMEOUT="${ASPEN_STAGE_PROCESS_TIMEOUT:-10}"
STAGE_NETWORK_TIMEOUT="${ASPEN_STAGE_NETWORK_TIMEOUT:-60}"

BRIDGE_NAME="aspen-br0"
BRIDGE_IP="10.100.0.1"
BRIDGE_SUBNET="24"

# Binaries (set by flake.nix wrapper)
ASPEN_NODE_BIN="${ASPEN_NODE_BIN:-}"
ASPEN_CLI_BIN="${ASPEN_CLI_BIN:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Track VM and virtiofsd PIDs for cleanup
declare -a VM_PIDS=()
declare -a VIRTIOFSD_PIDS=()

# Cleanup function
cleanup() {
    printf "\n${BLUE}Cleaning up VMs...${NC}\n"

    # Stop all VM processes first (they depend on virtiofsd)
    # VMs now run as current user (no sudo needed)
    for pid in "${VM_PIDS[@]}"; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            printf "  Stopping VM (PID %s)..." "$pid"
            kill "$pid" 2>/dev/null || true
            sleep 0.5
            if kill -0 "$pid" 2>/dev/null; then
                kill -9 "$pid" 2>/dev/null || true
            fi
            printf " ${GREEN}done${NC}\n"
        fi
    done

    # Stop virtiofsd processes (run as current user)
    for pid in "${VIRTIOFSD_PIDS[@]}"; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            printf "  Stopping virtiofsd (PID %s)..." "$pid"
            kill "$pid" 2>/dev/null || true
            sleep 0.3
            if kill -0 "$pid" 2>/dev/null; then
                kill -9 "$pid" 2>/dev/null || true
            fi
            printf " ${GREEN}done${NC}\n"
        fi
    done

    # Clean up any orphaned microvm/virtiofsd processes (all run as current user)
    # Use -f pattern matching to find processes by their exec name
    pkill -f "microvm@aspen-node" 2>/dev/null || true
    pkill -f "virtiofsd.*aspen-node" 2>/dev/null || true
    pkill -f "supervisord.*aspen-node" 2>/dev/null || true

    # Remove virtiofs sockets from project directory and /tmp
    rm -f aspen-node-*-virtiofs-*.sock* cloud-hypervisor.sock supervisord.pid 2>/dev/null || true
    rm -f /tmp/aspen-node-*-virtiofs-*.sock* 2>/dev/null || true

    # Clean up serial console logs (now owned by current user since cloud-hypervisor runs as user)
    rm -f /tmp/aspen-node-*-serial.log 2>/dev/null || true

    # Clean up VM state directory
    if [ -d "$VM_DIR" ]; then
        rm -rf "$VM_DIR"
        printf "  Removed %s\n" "$VM_DIR"
    fi

    printf "${GREEN}Cleanup complete${NC}\n"
}

trap cleanup EXIT INT TERM QUIT

# Check prerequisites
check_prerequisites() {
    # Check for KVM
    if [ ! -e /dev/kvm ]; then
        printf "${RED}Error: KVM not available.${NC}\n"
        printf "Please ensure KVM is enabled in your system.\n"
        exit 1
    fi

    # Check binaries
    if [ -z "$ASPEN_CLI_BIN" ] || [ ! -x "$ASPEN_CLI_BIN" ]; then
        ASPEN_CLI_BIN=$(find_binary aspen-cli)
    fi

    if [ -z "$ASPEN_CLI_BIN" ]; then
        printf "${RED}Error: aspen-cli not found.${NC}\n"
        printf "Run from flake: nix run .#dogfood-vm\n"
        exit 1
    fi
}

# Set up network bridge (requires sudo)
setup_network() {
    printf "${BLUE}Setting up network bridge...${NC}\n"

    if ! ip link show "$BRIDGE_NAME" &>/dev/null; then
        printf "  Creating bridge %s (%s/%s)...\n" "$BRIDGE_NAME" "$BRIDGE_IP" "$BRIDGE_SUBNET"

        if ! sudo -n true 2>/dev/null; then
            printf "  ${YELLOW}sudo required for network setup${NC}\n"
        fi

        sudo ip link add "$BRIDGE_NAME" type bridge
        sudo ip addr add "$BRIDGE_IP/$BRIDGE_SUBNET" dev "$BRIDGE_NAME"
        sudo ip link set "$BRIDGE_NAME" up

        # Enable IP forwarding for VM network
        sudo sysctl -w net.ipv4.ip_forward=1 >/dev/null

        printf "  ${GREEN}Bridge created${NC}\n"
    else
        printf "  Bridge %s already exists\n" "$BRIDGE_NAME"
    fi

    # Create TAP devices for each node
    # Note: devices are created owned by current user so cloud-hypervisor can access them
    # If TAP already exists with correct user ownership, reuse it (no sudo needed)
    for node_id in $(seq 1 "$NODE_COUNT"); do
        local tap_name="aspen-${node_id}"

        if ip link show "$tap_name" &>/dev/null; then
            # Check if TAP device has correct user ownership by testing if we can open it
            if [ -e "/dev/net/tun" ] && cat /sys/class/net/"$tap_name"/tun_flags &>/dev/null; then
                printf "  Reusing existing TAP device %s\n" "$tap_name"
                continue
            fi
            # TAP exists but may have wrong permissions, delete and recreate
            sudo ip link delete "$tap_name" 2>/dev/null || true
        fi

        printf "  Creating TAP device %s for user %s...\n" "$tap_name" "$USER"
        # Create TAP with user ownership, multi_queue for performance, vnet_hdr for virtio
        sudo ip tuntap add "$tap_name" mode tap user "$USER" vnet_hdr multi_queue
        sudo ip link set "$tap_name" master "$BRIDGE_NAME"
        sudo ip link set "$tap_name" up
    done

    printf "  ${GREEN}Network setup complete${NC}\n\n"
}

# Build VM image for a node
build_vm() {
    local node_id="$1"
    local vm_runner="$VM_DIR/vm-${node_id}/microvm-run"

    if [ -f "$vm_runner" ]; then
        printf "  Using cached VM image for node %d\n" "$node_id"
        return 0
    fi

    printf "  Building VM image for node %d..." "$node_id"

    # Build the VM using microvm.nix
    # Uses nixpkgs.lib.nixosSystem with microvm modules
    # The runner is at .config.microvm.runner.cloud-hypervisor
    if ! nix build \
        --no-link \
        --out-link "$VM_DIR/vm-${node_id}" \
        --impure \
        --expr "
          let
            flake = builtins.getFlake \"$PROJECT_DIR\";
            nixpkgs = flake.inputs.nixpkgs;
            microvm = flake.inputs.microvm;
            pkgs = import nixpkgs { system = \"x86_64-linux\"; };
          in
            (nixpkgs.lib.nixosSystem {
              system = \"x86_64-linux\";
              modules = [
                microvm.nixosModules.microvm
                $PROJECT_DIR/nix/modules/aspen-node.nix
                (import $PROJECT_DIR/nix/vms/dogfood-node.nix {
                  inherit (pkgs) lib;
                  nodeId = $node_id;
                  cookie = \"$COOKIE\";
                  aspenPackage = flake.packages.x86_64-linux.aspen-node;
                })
              ];
            }).config.microvm.runner.cloud-hypervisor
        " 2>"$VM_DIR/build-${node_id}.log"; then
        printf " ${RED}failed${NC}\n"
        printf "  See %s for details\n" "$VM_DIR/build-${node_id}.log"
        return 1
    fi

    printf " ${GREEN}done${NC}\n"
}

# Start virtiofsd for a VM (must run before the VM itself)
start_virtiofsd() {
    local node_id="$1"
    local virtiofsd_runner="$VM_DIR/vm-${node_id}/bin/virtiofsd-run"
    local virtiofsd_log="$VM_DIR/virtiofsd-${node_id}.log"
    # Socket path must match dogfood-node.nix configuration (absolute path in /tmp)
    local socket_name="/tmp/aspen-node-${node_id}-virtiofs-nix-store.sock"

    if [ ! -x "$virtiofsd_runner" ]; then
        printf "${RED}Error: virtiofsd runner not found for node %d${NC}\n" "$node_id"
        return 1
    fi

    # Start virtiofsd (supervisord manages the actual virtiofsd process)
    "$virtiofsd_runner" > "$virtiofsd_log" 2>&1 &
    local pid=$!
    VIRTIOFSD_PIDS+=("$pid")
    printf '%s\n' "$pid" > "$VM_DIR/virtiofsd-${node_id}.pid"

    # Wait for the virtiofs socket to be created (max 10 seconds)
    local elapsed=0
    while [ "$elapsed" -lt 10 ]; do
        if [ -S "$socket_name" ]; then
            return 0
        fi
        sleep 0.5
        elapsed=$((elapsed + 1))
    done

    printf "${RED}Error: virtiofs socket not created for node %d${NC}\n" "$node_id"
    return 1
}

# Start a VM
start_vm() {
    local node_id="$1"
    local vm_runner="$VM_DIR/vm-${node_id}/bin/microvm-run"
    local log_file="$VM_DIR/node-${node_id}.log"
    local serial_log="/tmp/aspen-node-${node_id}-serial.log"
    local process_name="microvm@aspen-node-${node_id}"

    if [ ! -x "$vm_runner" ]; then
        printf "${RED}Error: VM runner not found for node %d${NC}\n" "$node_id"
        return 1
    fi

    # Start virtiofsd first (provides /nix/store to the VM)
    if ! start_virtiofsd "$node_id"; then
        return 1
    fi

    printf "  Starting VM %d..." "$node_id"

    # Cloud Hypervisor runs as current user (no sudo needed):
    # - TAP devices are created with user ownership
    # - Landlock is disabled (can't sandbox sysfs)
    # - sysfs tun_flags are world-readable (444)
    # Start the VM in background with proper output redirection
    sh -c "\"$vm_runner\" > \"$log_file\" 2>&1 &"

    # Give the process a moment to start and exec into cloud-hypervisor
    sleep 1

    # Find the actual cloud-hypervisor process by its -a name (microvm@aspen-node-N)
    # This is more reliable than tracking the shell wrapper PID
    local ch_pid
    ch_pid=$(pgrep -f "$process_name" 2>/dev/null | head -1 || true)

    if [ -n "$ch_pid" ]; then
        VM_PIDS+=("$ch_pid")
        printf '%s\n' "$ch_pid" > "$VM_DIR/vm-${node_id}.pid"
        printf " PID %d (log: %s)\n" "$ch_pid" "$log_file"
    else
        # Could not find cloud-hypervisor process - VM likely failed to start
        # Check the log file for errors
        printf " ${RED}FAILED${NC} (process not found)\n"
        if [ -f "$log_file" ]; then
            printf "    ${YELLOW}=== VM startup error ===${NC}\n"
            tail -5 "$log_file" 2>/dev/null | sed 's/^/    /'
        fi
        return 1
    fi
}

# Check if VM process is running
# Uses pgrep to find the actual cloud-hypervisor process by name
# VMs run as current user, no sudo needed
is_vm_running() {
    local node_id="$1"
    local pid_file="$VM_DIR/vm-${node_id}.pid"
    local process_name="microvm@aspen-node-${node_id}"

    # First try the PID file
    if [ -f "$pid_file" ]; then
        local pid
        pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
    fi

    # Fallback: check by process name pattern (more reliable)
    if pgrep -f "$process_name" >/dev/null 2>&1; then
        # Update PID file with actual PID
        local ch_pid
        ch_pid=$(pgrep -f "$process_name" 2>/dev/null | head -1)
        if [ -n "$ch_pid" ]; then
            printf '%s\n' "$ch_pid" > "$pid_file"
        fi
        return 0
    fi

    return 1
}

# Wait for a VM to boot with multi-stage progress tracking
# Provides visibility into which stage failed for debugging
wait_for_vm_boot() {
    local node_id="$1"
    local ip="10.100.0.$((10 + node_id))"
    local pid_file="$VM_DIR/vm-${node_id}.pid"
    local serial_log="/tmp/aspen-node-${node_id}-serial.log"
    local process_name="microvm@aspen-node-${node_id}"

    printf "  Boot progress for node %d (%s):\n" "$node_id" "$ip"

    # Stage 1: Process started
    printf "    [1/3] VM process started... "
    local elapsed=0
    while [ "$elapsed" -lt "$STAGE_PROCESS_TIMEOUT" ]; do
        if is_vm_running "$node_id"; then
            local pid
            pid=$(cat "$pid_file" 2>/dev/null || pgrep -f "$process_name" | head -1)
            printf "${GREEN}OK${NC} (PID: %s)\n" "$pid"
            break
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    if [ "$elapsed" -ge "$STAGE_PROCESS_TIMEOUT" ]; then
        printf "${RED}FAILED${NC}\n"
        printf "    ${YELLOW}Diagnostic: VM process failed to spawn${NC}\n"
        printf "    ${YELLOW}Check: %s/node-%d.log${NC}\n" "$VM_DIR" "$node_id"
        # Show VM log if it exists
        if [ -f "$VM_DIR/node-${node_id}.log" ]; then
            printf "    ${YELLOW}=== Last 10 lines of VM log ===${NC}\n"
            tail -10 "$VM_DIR/node-${node_id}.log" 2>/dev/null | sed 's/^/    /'
        fi
        return 1
    fi

    # Create symlink to serial log for easy access
    if [ -f "$serial_log" ] || [ ! -e "$VM_DIR/serial-${node_id}.log" ]; then
        ln -sf "$serial_log" "$VM_DIR/serial-${node_id}.log" 2>/dev/null || true
    fi

    # Stage 2: Network reachable
    printf "    [2/3] Network reachable... "
    elapsed=0
    while [ "$elapsed" -lt "$STAGE_NETWORK_TIMEOUT" ]; do
        if ping -c 1 -W 1 "$ip" >/dev/null 2>&1; then
            printf "${GREEN}OK${NC} (%ds)\n" "$elapsed"
            break
        fi
        # Check if VM process died (use our reliable detection function)
        if ! is_vm_running "$node_id"; then
            printf "${RED}FAILED${NC} (VM process exited)\n"
            printf "    ${YELLOW}Check VM log: %s/node-%d.log${NC}\n" "$VM_DIR" "$node_id"
            show_serial_on_failure "$node_id"
            return 1
        fi
        # Progress indicator every 10s
        if [ $((elapsed % 10)) -eq 0 ] && [ "$elapsed" -gt 0 ]; then
            printf "%ds..." "$elapsed"
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    if [ "$elapsed" -ge "$STAGE_NETWORK_TIMEOUT" ]; then
        printf "${RED}TIMEOUT${NC} (%ds)\n" "$elapsed"
        print_network_diagnostics "$node_id"
        show_serial_on_failure "$node_id"
        return 1
    fi

    # Stage 3: Service ready (basic check - ping works means network is up)
    printf "    [3/3] Service ready... ${GREEN}OK${NC}\n"

    return 0
}

# Legacy function for backward compatibility
wait_for_vm() {
    wait_for_vm_boot "$@"
}

# Get ticket from node 1
get_ticket() {
    local ip="10.100.0.11"
    local log_file="$VM_DIR/node-1.log"
    local timeout=60
    local elapsed=0

    printf "  Waiting for cluster ticket..."

    while [ "$elapsed" -lt "$timeout" ]; do
        # Try to get ticket from log
        if [ -f "$log_file" ]; then
            local ticket
            ticket=$(grep -oE 'aspen[a-z2-7]{50,200}' "$log_file" 2>/dev/null | head -1 || true)

            if [ -n "$ticket" ]; then
                printf " ${GREEN}found${NC}\n"
                printf '%s' "$ticket" > "$VM_DIR/ticket.txt"
                echo "$ticket"
                return 0
            fi
        fi

        sleep 1
        elapsed=$((elapsed + 1))
        printf "."
    done

    printf " ${RED}timeout${NC}\n"
    return 1
}

# Initialize the cluster
init_cluster() {
    local ticket="$1"

    printf "${BLUE}Initializing cluster...${NC}\n"

    # Wait for gossip discovery
    printf "  Waiting for gossip discovery..."
    sleep 5
    printf " ${GREEN}done${NC}\n"

    # Initialize node 1
    printf "  Initializing Raft on node 1..."
    if retry_with_backoff 5 1 8 "$ASPEN_CLI_BIN" --ticket "$ticket" cluster init; then
        printf " ${GREEN}done${NC}\n"
    else
        printf " ${RED}failed${NC}\n"
        return 1
    fi

    sleep 2

    # Add other nodes as learners
    if [ "$NODE_COUNT" -gt 1 ]; then
        printf "  Adding nodes 2-%d as learners...\n" "$NODE_COUNT"

        for node_id in $(seq 2 "$NODE_COUNT"); do
            local log_file="$VM_DIR/node-${node_id}.log"
            local endpoint_id

            # Extract endpoint ID from log
            endpoint_id=$(sed 's/\x1b\[[0-9;]*m//g' "$log_file" 2>/dev/null | \
                grep -oE 'endpoint_id=[a-f0-9]{64}' | head -1 | cut -d= -f2 || true)

            if [ -n "$endpoint_id" ]; then
                printf "    Node %d (%s...): " "$node_id" "$(echo "$endpoint_id" | cut -c1-16)"
                if retry_with_backoff 3 1 4 "$ASPEN_CLI_BIN" --ticket "$ticket" cluster add-learner \
                    --node-id "$node_id" --addr "$endpoint_id"; then
                    printf "${GREEN}added${NC}\n"
                else
                    printf "${YELLOW}skipped${NC}\n"
                fi
            else
                printf "    Node %d: ${YELLOW}no endpoint ID yet${NC}\n" "$node_id"
            fi
            sleep 1
        done

        # Promote all to voters
        printf "  Promoting all nodes to voters..."

        local members=""
        for id in $(seq 1 "$NODE_COUNT"); do
            members="${members:+$members }$id"
        done

        sleep 2
        if retry_with_backoff 3 1 4 "$ASPEN_CLI_BIN" --ticket "$ticket" cluster change-membership $members; then
            printf " ${GREEN}done${NC}\n"
        else
            printf " ${YELLOW}skipped (may already be voters)${NC}\n"
        fi
    fi

    # Wait for cluster to stabilize
    printf "  Waiting for cluster to stabilize..."
    if wait_for_cluster_stable "$ASPEN_CLI_BIN" "$ticket" 10000 60; then
        printf " ${GREEN}done${NC}\n"
    else
        printf " ${YELLOW}may still be stabilizing${NC}\n"
    fi

    return 0
}

# Create Forge repository
init_repo() {
    local ticket="$1"

    printf "${BLUE}Creating Aspen repository in Forge...${NC}\n"

    local output
    if ! output=$("$ASPEN_CLI_BIN" --ticket "$ticket" git init \
        --description "Aspen distributed systems platform (self-hosted in VMs)" \
        "aspen" 2>&1); then
        printf "  ${RED}Failed: %s${NC}\n" "$output"
        return 1
    fi

    local repo_id
    repo_id=$(echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || true)

    if [ -z "$repo_id" ]; then
        printf "  ${YELLOW}Repository may already exist${NC}\n"
        return 1
    fi

    printf '%s' "$repo_id" > "$VM_DIR/repo_id.txt"
    printf "  Repo ID: %s\n" "$repo_id"

    # Configure git remote
    local remote_url="aspen://$ticket/$repo_id"
    if git remote get-url aspen >/dev/null 2>&1; then
        git remote set-url aspen "$remote_url"
    else
        git remote add aspen "$remote_url"
    fi
    printf "  Git remote configured\n"

    # Enable CI watching
    printf "  Enabling CI auto-trigger..."
    if "$ASPEN_CLI_BIN" --ticket "$ticket" ci watch "$repo_id" >/dev/null 2>&1; then
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

    # Find git-remote-aspen
    local git_remote_bin
    git_remote_bin=$(find_binary git-remote-aspen)

    if [ -z "$git_remote_bin" ]; then
        printf "  ${RED}git-remote-aspen not found${NC}\n"
        return 1
    fi

    local bin_dir
    bin_dir=$(dirname "$git_remote_bin")

    printf "  Branch: %s\n" "$branch"
    PATH="$bin_dir:$PATH" git push aspen "$branch"

    printf "  ${GREEN}Push complete${NC}\n"
}

# Print cluster info
print_info() {
    local ticket="$1"

    printf "\n${BLUE}======================================${NC}\n"
    printf "${GREEN}Aspen VM Cluster Ready${NC} (%d nodes)\n" "$NODE_COUNT"
    printf "${BLUE}======================================${NC}\n"
    printf "\n"
    printf "Cookie:     %s\n" "$COOKIE"
    printf "Network:    10.100.0.0/24 (bridge: %s)\n" "$BRIDGE_NAME"
    printf "VMs:        %s\n" "$VM_DIR"
    printf "Ticket:     %s/ticket.txt\n" "$VM_DIR"
    printf "\n"

    printf "${BLUE}Node IPs:${NC}\n"
    for node_id in $(seq 1 "$NODE_COUNT"); do
        printf "  Node %d: 10.100.0.%d\n" "$node_id" "$((10 + node_id))"
    done
    printf "\n"

    printf "${BLUE}CLI Commands:${NC}\n"
    printf "  %s --ticket %s cluster status\n" "$ASPEN_CLI_BIN" "$ticket"
    printf "  %s --ticket %s ci status\n" "$ASPEN_CLI_BIN" "$ticket"
    printf "\n"

    printf "${BLUE}Debug Logs:${NC}\n"
    printf "  Serial console:  nix run .#dogfood-vm -- serial 1\n"
    printf "                   tail -f /tmp/aspen-node-1-serial.log\n"
    printf "  VM stdout:       tail -f %s/node-1.log\n" "$VM_DIR"
    printf "  Build log:       %s/build-1.log\n" "$VM_DIR"
    printf "  virtiofsd log:   %s/virtiofsd-1.log\n" "$VM_DIR"
    printf "\n"

    printf "${BLUE}Stop cluster:${NC}\n"
    printf "  Press Ctrl+C\n"
    printf "${BLUE}======================================${NC}\n"
}

# Run complete workflow
cmd_run() {
    local branch="${1:-main}"

    printf "${BLUE}============================================${NC}\n"
    printf "${GREEN}Aspen Self-Hosting in VMs${NC}\n"
    printf "${BLUE}============================================${NC}\n\n"

    check_prerequisites
    mkdir -p "$VM_DIR"

    # Clean leftover sockets from previous runs (prevents nix build failures)
    # Nix flakes can't copy Unix socket files, so stale sockets break builds
    rm -f aspen-node-*-virtiofs-*.sock* supervisord.pid cloud-hypervisor.sock 2>/dev/null
    rm -f /tmp/aspen-node-*-virtiofs-*.sock* 2>/dev/null

    # Step 1: Network setup
    setup_network

    # Step 2: Build VMs
    printf "${BLUE}Building VM images...${NC}\n"
    for node_id in $(seq 1 "$NODE_COUNT"); do
        if ! build_vm "$node_id"; then
            printf "${RED}Failed to build VM for node %d${NC}\n" "$node_id"
            exit 1
        fi
    done
    printf "\n"

    # Step 3: Start VMs
    printf "${BLUE}Starting VMs...${NC}\n"
    for node_id in $(seq 1 "$NODE_COUNT"); do
        start_vm "$node_id"
    done
    printf "\n"

    # Step 4: Wait for VMs
    printf "${BLUE}Waiting for VMs to boot...${NC}\n"
    for node_id in $(seq 1 "$NODE_COUNT"); do
        if ! wait_for_vm "$node_id" 120; then
            printf "${RED}VM %d did not become reachable${NC}\n" "$node_id"
            exit 1
        fi
    done
    printf "\n"

    # Step 5: Get ticket and init cluster
    local ticket
    if ! ticket=$(get_ticket); then
        printf "${RED}Failed to get cluster ticket${NC}\n"
        exit 1
    fi

    if ! init_cluster "$ticket"; then
        printf "${RED}Cluster initialization failed${NC}\n"
        exit 1
    fi
    printf "\n"

    # Step 6: Create repo and push
    if ! init_repo "$ticket"; then
        printf "${YELLOW}Repository setup failed (may already exist)${NC}\n"
    fi

    push_to_aspen "$branch"
    printf "\n"

    # Print info and wait
    print_info "$ticket"

    printf "\n${YELLOW}Running in foreground. Press Ctrl+C to stop.${NC}\n"

    # Monitor VMs
    while true; do
        local all_running=true
        for pid in "${VM_PIDS[@]}"; do
            if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
                all_running=false
                break
            fi
        done

        if [ "$all_running" = "false" ]; then
            printf "${RED}A VM died unexpectedly${NC}\n"
            exit 1
        fi

        sleep 5
    done
}

# Show cluster status
cmd_status() {
    if [ ! -d "$VM_DIR" ]; then
        printf "${YELLOW}No VM cluster running${NC}\n"
        return 1
    fi

    printf "${BLUE}VM Cluster Status${NC}\n\n"

    local running=0
    for node_id in $(seq 1 "$NODE_COUNT"); do
        local pid_file="$VM_DIR/vm-${node_id}.pid"
        if [ -f "$pid_file" ]; then
            local pid
            pid=$(cat "$pid_file")
            local ip="10.100.0.$((10 + node_id))"

            if kill -0 "$pid" 2>/dev/null; then
                if ping -c 1 -W 1 "$ip" >/dev/null 2>&1; then
                    printf "  Node %d: ${GREEN}running${NC} (PID %s, IP %s)\n" "$node_id" "$pid" "$ip"
                else
                    printf "  Node %d: ${YELLOW}starting${NC} (PID %s, IP %s)\n" "$node_id" "$pid" "$ip"
                fi
                running=$((running + 1))
            else
                printf "  Node %d: ${RED}stopped${NC}\n" "$node_id"
            fi
        fi
    done

    if [ -f "$VM_DIR/ticket.txt" ]; then
        printf "\nTicket: %s\n" "$(cat "$VM_DIR/ticket.txt")"
    fi

    [ "$running" -gt 0 ]
}

# Stop all VMs
cmd_stop() {
    printf "${BLUE}Stopping VM cluster...${NC}\n"
    cleanup
}

# Follow serial console output for a VM (real-time)
cmd_serial() {
    local node_id="${1:-1}"
    local serial_log="/tmp/aspen-node-${node_id}-serial.log"

    if [ ! -f "$serial_log" ]; then
        printf "${RED}Serial log not found for node %d${NC}\n" "$node_id"
        printf "Expected: %s\n" "$serial_log"
        printf "\nMake sure VMs are running: nix run .#dogfood-vm -- status\n"
        return 1
    fi

    printf "${BLUE}Serial console for node %d${NC}\n" "$node_id"
    printf "File: %s\n" "$serial_log"
    printf "Press Ctrl+C to stop\n\n"
    tail -f "$serial_log"
}

# Show serial console logs (dump, not follow)
cmd_logs() {
    local node_id="${1:-}"

    if [ -n "$node_id" ]; then
        local serial_log="/tmp/aspen-node-${node_id}-serial.log"
        if [ -f "$serial_log" ]; then
            printf "${BLUE}=== Serial Console for Node %d ===${NC}\n" "$node_id"
            cat "$serial_log"
        else
            printf "${RED}No serial log for node %d${NC}\n" "$node_id"
            printf "Expected: %s\n" "$serial_log"
        fi
    else
        # Show all available logs
        local found=0
        for log in /tmp/aspen-node-*-serial.log; do
            if [ -f "$log" ]; then
                local id
                id=$(echo "$log" | grep -oE 'node-[0-9]+' | grep -oE '[0-9]+')
                printf "\n${BLUE}=== Serial Console for Node %s ===${NC}\n" "$id"
                cat "$log"
                found=1
            fi
        done
        if [ "$found" -eq 0 ]; then
            printf "${YELLOW}No serial logs found${NC}\n"
            printf "Make sure VMs are running: nix run .#dogfood-vm -- status\n"
        fi
    fi
}

# Show help
cmd_help() {
    printf "Aspen Self-Hosting in Cloud Hypervisor MicroVMs\n"
    printf "\n"
    printf "Usage: nix run .#dogfood-vm [-- command]\n"
    printf "\n"
    printf "Commands:\n"
    printf "  run        - ${GREEN}Full workflow${NC} (start VMs, init cluster, push)\n"
    printf "  status     - Show VM cluster status\n"
    printf "  stop       - Stop all VMs\n"
    printf "  serial [n] - Follow serial console for node n (default: 1)\n"
    printf "  logs [n]   - Show serial logs for node n (or all if omitted)\n"
    printf "  help       - Show this help message\n"
    printf "\n"
    printf "Environment variables:\n"
    printf "  ASPEN_NODE_COUNT              - Number of VMs (default: 3, max: 10)\n"
    printf "  ASPEN_VM_DIR                  - VM state directory (default: .aspen/vms)\n"
    printf "  ASPEN_LOG_LEVEL               - Log level (default: info)\n"
    printf "  ASPEN_STAGE_PROCESS_TIMEOUT   - Process start timeout (default: 10s)\n"
    printf "  ASPEN_STAGE_NETWORK_TIMEOUT   - Network reachable timeout (default: 60s)\n"
    printf "\n"
    printf "Examples:\n"
    printf "  nix run .#dogfood-vm                        # Default 3-node cluster\n"
    printf "  ASPEN_NODE_COUNT=5 nix run .#dogfood-vm     # 5-node cluster\n"
    printf "  ASPEN_NODE_COUNT=1 nix run .#dogfood-vm     # Single-node for testing\n"
    printf "\n"
    printf "Requirements:\n"
    printf "  - KVM enabled (/dev/kvm accessible)\n"
    printf "  - First run: sudo for network bridge and TAP device setup\n"
    printf "  - Subsequent runs: no sudo needed (reuses existing devices)\n"
    printf "\n"
}

# Main
main() {
    local cmd="${1:-run}"
    shift || true

    case "$cmd" in
        run)
            cmd_run "$@"
            ;;
        status)
            cmd_status
            ;;
        stop)
            cmd_stop
            ;;
        serial)
            cmd_serial "$@"
            ;;
        logs)
            cmd_logs "$@"
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
