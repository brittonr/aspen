#!/usr/bin/env bash
# Setup TAP network interfaces for MicroVM testing
# This script creates the network infrastructure needed for Cloud Hypervisor VMs
#
# Usage:
#   sudo ./setup-microvm-network.sh           # Setup network
#   sudo ./setup-microvm-network.sh teardown  # Remove network

set -e

# Configuration
BRIDGE_NAME="vm-br0"
BRIDGE_IP="192.168.100.1/24"
TAP_PREFIX="vm-net"
NUM_TAPS=10
TAP_USER="${SUDO_USER:-$USER}"  # Use the user who called sudo, or current user

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

setup_bridge() {
    if ip link show "$BRIDGE_NAME" &>/dev/null; then
        log_warn "Bridge $BRIDGE_NAME already exists"
    else
        log_info "Creating bridge $BRIDGE_NAME"
        ip link add "$BRIDGE_NAME" type bridge
        ip addr add "$BRIDGE_IP" dev "$BRIDGE_NAME"
        ip link set "$BRIDGE_NAME" up
    fi
}

create_tap() {
    local tap_name=$1
    local tap_user=$2

    if ip link show "$tap_name" &>/dev/null; then
        log_warn "TAP interface $tap_name already exists"
    else
        log_info "Creating TAP interface $tap_name for user $tap_user"
        ip tuntap add name "$tap_name" mode tap user "$tap_user"
        ip link set "$tap_name" up
        ip link set "$tap_name" master "$BRIDGE_NAME"
    fi
}

setup_nat() {
    log_info "Setting up NAT for VM subnet"

    # Enable IP forwarding
    sysctl -w net.ipv4.ip_forward=1 > /dev/null

    # Get default interface (the one with default route)
    DEFAULT_IF=$(ip route | grep default | awk '{print $5}' | head -n1)

    if [ -z "$DEFAULT_IF" ]; then
        log_error "Could not determine default network interface"
        exit 1
    fi

    log_info "Using $DEFAULT_IF as external interface"

    # Setup NAT rules
    iptables -t nat -C POSTROUTING -s 192.168.100.0/24 -o "$DEFAULT_IF" -j MASQUERADE 2>/dev/null || \
        iptables -t nat -A POSTROUTING -s 192.168.100.0/24 -o "$DEFAULT_IF" -j MASQUERADE

    iptables -C FORWARD -i "$BRIDGE_NAME" -j ACCEPT 2>/dev/null || \
        iptables -A FORWARD -i "$BRIDGE_NAME" -j ACCEPT

    iptables -C FORWARD -o "$BRIDGE_NAME" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || \
        iptables -A FORWARD -o "$BRIDGE_NAME" -m state --state RELATED,ESTABLISHED -j ACCEPT
}

setup_network() {
    log_info "Setting up MicroVM network infrastructure"

    # Create bridge
    setup_bridge

    # Create TAP interfaces
    for i in $(seq 0 $((NUM_TAPS - 1))); do
        create_tap "${TAP_PREFIX}${i}" "$TAP_USER"
    done

    # Setup NAT
    setup_nat

    log_info "Network setup complete!"
    log_info "Bridge: $BRIDGE_NAME ($BRIDGE_IP)"
    log_info "TAP interfaces: ${TAP_PREFIX}0 to ${TAP_PREFIX}$((NUM_TAPS - 1))"
    log_info "TAP owner: $TAP_USER"
}

teardown_network() {
    log_info "Tearing down MicroVM network infrastructure"

    # Remove TAP interfaces
    for i in $(seq 0 $((NUM_TAPS - 1))); do
        tap_name="${TAP_PREFIX}${i}"
        if ip link show "$tap_name" &>/dev/null; then
            log_info "Removing TAP interface $tap_name"
            ip link delete "$tap_name"
        fi
    done

    # Remove bridge
    if ip link show "$BRIDGE_NAME" &>/dev/null; then
        log_info "Removing bridge $BRIDGE_NAME"
        ip link delete "$BRIDGE_NAME"
    fi

    # Remove NAT rules
    DEFAULT_IF=$(ip route | grep default | awk '{print $5}' | head -n1)
    if [ -n "$DEFAULT_IF" ]; then
        log_info "Removing NAT rules"
        iptables -t nat -D POSTROUTING -s 192.168.100.0/24 -o "$DEFAULT_IF" -j MASQUERADE 2>/dev/null || true
        iptables -D FORWARD -i "$BRIDGE_NAME" -j ACCEPT 2>/dev/null || true
        iptables -D FORWARD -o "$BRIDGE_NAME" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
    fi

    log_info "Network teardown complete"
}

show_status() {
    echo "=== MicroVM Network Status ==="
    echo ""

    echo "Bridge:"
    if ip link show "$BRIDGE_NAME" &>/dev/null; then
        ip addr show "$BRIDGE_NAME" | grep -E "inet |state"
    else
        echo "  Bridge $BRIDGE_NAME not found"
    fi
    echo ""

    echo "TAP interfaces:"
    for i in $(seq 0 $((NUM_TAPS - 1))); do
        tap_name="${TAP_PREFIX}${i}"
        if ip link show "$tap_name" &>/dev/null; then
            echo -n "  $tap_name: "
            ip link show "$tap_name" | grep -o "state [A-Z]*" || echo "exists"
        fi
    done
    echo ""

    echo "NAT rules:"
    if iptables -t nat -L POSTROUTING -n | grep -q "192.168.100.0/24"; then
        echo "  NAT enabled for 192.168.100.0/24"
    else
        echo "  No NAT rules found"
    fi
}

# Main script
check_root

case "${1:-setup}" in
    setup)
        setup_network
        ;;
    teardown|down)
        teardown_network
        ;;
    status)
        show_status
        ;;
    *)
        echo "Usage: $0 [setup|teardown|status]"
        echo ""
        echo "  setup    - Create bridge and TAP interfaces (default)"
        echo "  teardown - Remove all network infrastructure"
        echo "  status   - Show current network status"
        exit 1
        ;;
esac