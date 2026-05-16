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
# The network configuration persists until reboot. The helper copy under
# /usr/local/libexec is recreated by this script after reboot. Run this once
# before using nix run .#dogfood-local-vmci to avoid interactive sudo prompts.

set -eu

# Configuration (must match dogfood-local-vmci.sh)
BRIDGE_NAME="aspen-ci-br0"
BRIDGE_IP="10.200.0.1/24"
NODE_COUNT="${ASPEN_NODE_COUNT:-1}"
TAP_USER="${SUDO_USER:-$USER}"
TAP_HELPER_SOURCE="${ASPEN_CI_TAP_HELPER_SOURCE:-}"
TAP_HELPER_PATH="${ASPEN_CI_TAP_HELPER_PATH:-/usr/local/libexec/aspen-ci-tap-helper}"

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

# Install the narrow TAP helper used by unprivileged aspen-node. The Nix store
# binary is immutable, so copy it to a mutable root-owned path before applying
# file capabilities.
if [ -n "$TAP_HELPER_SOURCE" ]; then
    printf "  Installing TAP helper..."
    install -D -m 0755 -o root -g root "$TAP_HELPER_SOURCE" "$TAP_HELPER_PATH"
    if command -v setcap >/dev/null 2>&1; then
        setcap cap_net_admin+ep "$TAP_HELPER_PATH"
        printf " ${GREEN}done${NC} (%s)\n" "$TAP_HELPER_PATH"
        if command -v findmnt >/dev/null 2>&1; then
            mount_options=$(findmnt -T "$TAP_HELPER_PATH" -no OPTIONS 2>/dev/null || true)
            case ",$mount_options," in
                *,nosuid,*)
                    printf "  ${YELLOW}Warning:${NC} helper path is on a nosuid mount; file capabilities may be ignored: %s\n" "$TAP_HELPER_PATH"
                    ;;
            esac
        fi
    else
        printf " ${YELLOW}installed without capabilities${NC} (setcap not found)\n"
    fi
else
    printf "  TAP helper source not provided; skipping helper install\n"
fi

# Create bridge if needed
if ip link show "$BRIDGE_NAME" >/dev/null 2>&1; then
    printf "  Bridge %s already exists\n" "$BRIDGE_NAME"
    if ! ip -4 addr show dev "$BRIDGE_NAME" | grep -qF "$BRIDGE_IP"; then
        ip addr add "$BRIDGE_IP" dev "$BRIDGE_NAME" 2>/dev/null || true
    fi
    ip link set "$BRIDGE_NAME" up
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

# Set up host ingress/forwarding and NAT using nftables (modern) with
# iptables fallback. NixOS firewalls commonly allow ICMP from the bridge while
# dropping UDP to host processes; VM workers need UDP/QUIC access to the local
# Aspen node at 10.200.0.1:<iroh-port>, so accept bridge ingress explicitly.
#
# Important nftables detail: an `accept` verdict in a separate base chain does
# not bypass later base chains on the same hook. NixOS installs its firewall in
# `inet nixos-fw input` with a later drop policy, so the VM bridge accept rule
# must also be inserted into that chain when present. Otherwise ping can work
# while guest->host Iroh/QUIC UDP times out.
printf "  Configuring VM firewall/NAT..."
if command -v nft >/dev/null 2>&1; then
    add_nft_rule_once() {
        family="$1"
        table="$2"
        chain="$3"
        comment="$4"
        shift 4

        if ! nft list chain "$family" "$table" "$chain" >/dev/null 2>&1; then
            return 0
        fi
        if nft list chain "$family" "$table" "$chain" | grep -q "comment \"$comment\""; then
            return 0
        fi
        nft insert rule "$family" "$table" "$chain" "$@" counter accept comment "\"$comment\""
    }

    if ! nft list table inet aspen-ci-filter >/dev/null 2>&1; then
        nft add table inet aspen-ci-filter
        nft add chain inet aspen-ci-filter input '{ type filter hook input priority -150 ; policy accept ; }'
        nft add chain inet aspen-ci-filter forward '{ type filter hook forward priority -150 ; policy accept ; }'
    fi
    add_nft_rule_once inet aspen-ci-filter input "aspen-ci bridge ingress" iifname "\"$BRIDGE_NAME\""
    add_nft_rule_once inet aspen-ci-filter forward "aspen-ci bridge forward ingress" iifname "\"$BRIDGE_NAME\""
    add_nft_rule_once inet aspen-ci-filter forward "aspen-ci bridge forward egress" oifname "\"$BRIDGE_NAME\""

    # NixOS' nftables firewall uses this table/chain and can drop packets after
    # the compatibility base chain above accepted them. Insert into the product
    # firewall chain as an idempotent host-local setup rule when it exists.
    add_nft_rule_once inet nixos-fw input "aspen-ci bridge ingress" iifname "\"$BRIDGE_NAME\""
    add_nft_rule_once inet nixos-fw forward "aspen-ci bridge forward ingress" iifname "\"$BRIDGE_NAME\""
    add_nft_rule_once inet nixos-fw forward "aspen-ci bridge forward egress" oifname "\"$BRIDGE_NAME\""

    if ! nft list table ip aspen-ci-nat >/dev/null 2>&1; then
        nft add table ip aspen-ci-nat
        nft add chain ip aspen-ci-nat postrouting '{ type nat hook postrouting priority 100 ; }'
    fi
    if ! nft list chain ip aspen-ci-nat postrouting | grep -q "10.200.0.0/24"; then
        nft add rule ip aspen-ci-nat postrouting ip saddr 10.200.0.0/24 oifname != "\"$BRIDGE_NAME\"" masquerade
    fi
    printf " ${GREEN}done${NC} (nftables)\n"
elif command -v iptables >/dev/null 2>&1; then
    iptables -C INPUT -i "$BRIDGE_NAME" -j ACCEPT 2>/dev/null || iptables -I INPUT -i "$BRIDGE_NAME" -j ACCEPT
    iptables -C FORWARD -i "$BRIDGE_NAME" -j ACCEPT 2>/dev/null || iptables -I FORWARD -i "$BRIDGE_NAME" -j ACCEPT
    iptables -C FORWARD -o "$BRIDGE_NAME" -j ACCEPT 2>/dev/null || iptables -I FORWARD -o "$BRIDGE_NAME" -j ACCEPT
    iptables -t nat -C POSTROUTING -s 10.200.0.0/24 ! -o "$BRIDGE_NAME" -j MASQUERADE 2>/dev/null \
        || iptables -t nat -A POSTROUTING -s 10.200.0.0/24 ! -o "$BRIDGE_NAME" -j MASQUERADE
    printf " ${GREEN}done${NC} (iptables)\n"
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
NETWORK_SETUP_MARKER="/tmp/aspen-ci-network-configured-v3"
touch "$NETWORK_SETUP_MARKER"
# Legacy markers for older wrappers that only checked NAT or pre-NixOS-firewall
# bridge setup. Current VM-CI readiness requires v3 so stale v2 hosts rerun this
# script and install the NixOS firewall-chain ingress rules above.
touch /tmp/aspen-ci-network-configured-v2 /tmp/aspen-ci-network-configured
chmod 644 "$NETWORK_SETUP_MARKER" /tmp/aspen-ci-network-configured-v2 /tmp/aspen-ci-network-configured

printf "\n${GREEN}Network setup complete!${NC}\n"
printf "\nYou can now run: ${BLUE}nix run .#dogfood-local-vmci${NC}\n"
printf "\nNote: This configuration persists until reboot.\n"
printf "To remove: ${BLUE}sudo ip link del %s${NC}\n" "$BRIDGE_NAME"
