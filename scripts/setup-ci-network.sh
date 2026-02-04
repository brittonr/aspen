#!/usr/bin/env bash
# Set up network bridge and TAP devices for CI VMs.
#
# This script must be run with sudo/root privileges. It creates:
# - A network bridge (aspen-ci-br0) for VM connectivity
# - NAT rules for internet access (cache.nixos.org)
# - TAP devices for each potential VM slot
#
# Usage:
#   sudo nix run .#setup-ci-network
#   # Or manually:
#   sudo ./scripts/setup-ci-network.sh
#
# The network configuration persists until reboot. Run this once before
# using nix run .#dogfood-local-vmci to avoid interactive sudo prompts.

set -eu

# Configuration (must match dogfood-local-vmci.sh)
BRIDGE_NAME="aspen-ci-br0"
BRIDGE_IP="10.200.0.1/24"
NODE_COUNT="${ASPEN_NODE_COUNT:-1}"
TAP_USER="${SUDO_USER:-$USER}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Check for root
if [ "$(id -u)" -ne 0 ]; then
    printf "${RED}Error: This script must be run as root (sudo)${NC}\n"
    printf "Usage: sudo %s\n" "$0"
    exit 1
fi

printf "${BLUE}Setting up CI VM network...${NC}\n\n"

# Create bridge if needed
if ip link show "$BRIDGE_NAME" >/dev/null 2>&1; then
    printf "  Bridge %s already exists\n" "$BRIDGE_NAME"
else
    printf "  Creating bridge %s..." "$BRIDGE_NAME"
    ip link add "$BRIDGE_NAME" type bridge
    ip addr add "$BRIDGE_IP" dev "$BRIDGE_NAME" 2>/dev/null || true
    ip link set "$BRIDGE_NAME" up
    printf " ${GREEN}done${NC}\n"
fi

# Enable IP forwarding
printf "  Enabling IP forwarding..."
sysctl -w net.ipv4.ip_forward=1 >/dev/null 2>&1
printf " ${GREEN}done${NC}\n"

# Set up NAT using nftables (modern) with iptables fallback
printf "  Configuring NAT..."
if command -v nft >/dev/null 2>&1; then
    if nft list table ip aspen-ci-nat >/dev/null 2>&1; then
        printf " already configured (nftables)\n"
    else
        nft add table ip aspen-ci-nat
        nft add chain ip aspen-ci-nat postrouting '{ type nat hook postrouting priority 100 ; }'
        nft add rule ip aspen-ci-nat postrouting ip saddr 10.200.0.0/24 oifname != "$BRIDGE_NAME" masquerade
        printf " ${GREEN}done${NC} (nftables)\n"
    fi
elif command -v iptables >/dev/null 2>&1; then
    if iptables -t nat -C POSTROUTING -s 10.200.0.0/24 ! -o "$BRIDGE_NAME" -j MASQUERADE 2>/dev/null; then
        printf " already configured (iptables)\n"
    else
        iptables -t nat -A POSTROUTING -s 10.200.0.0/24 ! -o "$BRIDGE_NAME" -j MASQUERADE
        printf " ${GREEN}done${NC} (iptables)\n"
    fi
else
    printf " ${YELLOW}skipped${NC} (no nftables or iptables)\n"
fi

# Create TAP devices for each VM slot
printf "  Creating TAP devices for user %s...\n" "$TAP_USER"
created=0
for node_id in $(seq 1 "$NODE_COUNT"); do
    for vm_idx in $(seq 0 7); do
        tap_name="ci-n${node_id}-vm${vm_idx}-tap"
        if ip link show "$tap_name" >/dev/null 2>&1; then
            continue
        fi
        if ip tuntap add "$tap_name" mode tap user "$TAP_USER" 2>/dev/null; then
            ip link set "$tap_name" master "$BRIDGE_NAME" 2>/dev/null || true
            ip link set "$tap_name" up 2>/dev/null || true
            created=$((created + 1))
        fi
    done
done

if [ "$created" -gt 0 ]; then
    printf "    Created %d TAP devices\n" "$created"
else
    printf "    TAP devices already exist\n"
fi

# Create marker file so dogfood script knows NAT is configured
# (nft/iptables require root to check rules)
NETWORK_SETUP_MARKER="/tmp/aspen-ci-network-configured"
touch "$NETWORK_SETUP_MARKER"
chmod 644 "$NETWORK_SETUP_MARKER"

printf "\n${GREEN}Network setup complete!${NC}\n"
printf "\nYou can now run: ${BLUE}nix run .#dogfood-local-vmci${NC}\n"
printf "\nNote: This configuration persists until reboot.\n"
printf "To remove: ${BLUE}sudo ip link del %s${NC}\n" "$BRIDGE_NAME"
