#!/bin/bash
# Network diagnostics for dogfood-vm
#
# This library provides functions for diagnosing network issues in VM-based
# cluster testing. Source this file to get network debugging utilities.
#
# Functions:
#   verify_tap_device      - Check TAP device exists and attached to bridge
#   check_arp_for_vm       - Check ARP table for VM MAC address
#   print_network_diagnostics - Print comprehensive diagnostics on failure

# Verify TAP device exists and is attached to bridge
# Usage: verify_tap_device <tap_name> <bridge_name>
# Returns: 0 if OK, 1 if failed
verify_tap_device() {
    local tap_name="$1"
    local bridge_name="$2"

    if ! ip link show "$tap_name" &>/dev/null; then
        printf "  ${RED}TAP device %s does not exist${NC}\n" "$tap_name"
        return 1
    fi

    # Check TAP is UP
    local state
    state=$(ip -o link show "$tap_name" 2>/dev/null | grep -oE 'state [A-Z]+' | cut -d' ' -f2)
    if [ "$state" != "UP" ]; then
        printf "  ${YELLOW}TAP device %s state is %s (expected UP)${NC}\n" "$tap_name" "$state"
    fi

    # Check TAP is attached to bridge
    local master
    master=$(ip -o link show "$tap_name" 2>/dev/null | grep -oE 'master [^ ]+' | cut -d' ' -f2)
    if [ "$master" != "$bridge_name" ]; then
        printf "  ${RED}TAP %s not attached to %s (master: %s)${NC}\n" "$tap_name" "$bridge_name" "${master:-none}"
        return 1
    fi

    return 0
}

# Check ARP table for VM MAC address
# Usage: check_arp_for_vm <node_id>
# Returns: 0 if MAC found, 1 if not found
# Note: Requires BRIDGE_NAME to be set
check_arp_for_vm() {
    local node_id="$1"
    local ip="10.100.0.$((10 + node_id))"
    local expected_mac
    expected_mac="02:00:00:01:01:$(printf '%02x' "$node_id")"

    local arp_entry
    arp_entry=$(ip neigh show "$ip" 2>/dev/null | head -1)

    if [ -n "$arp_entry" ]; then
        printf "  ARP: %s\n" "$arp_entry"
        return 0
    else
        printf "  ${YELLOW}No ARP entry for %s (expected MAC: %s)${NC}\n" "$ip" "$expected_mac"
        return 1
    fi
}

# Print comprehensive diagnostics on network failure
# Usage: print_network_diagnostics <node_id>
# Note: Requires BRIDGE_NAME and VM_DIR to be set
print_network_diagnostics() {
    local node_id="$1"
    local ip="10.100.0.$((10 + node_id))"
    local tap_name="aspen-${node_id}"

    printf "\n${BLUE}=== Network Diagnostics for Node %d ===${NC}\n\n" "$node_id"

    printf "${YELLOW}TAP Device (%s):${NC}\n" "$tap_name"
    ip link show "$tap_name" 2>&1 | sed 's/^/  /'
    printf "\n"

    printf "${YELLOW}Bridge (%s):${NC}\n" "$BRIDGE_NAME"
    ip addr show "$BRIDGE_NAME" 2>&1 | sed 's/^/  /'
    printf "\n"

    printf "${YELLOW}Bridge Members:${NC}\n"
    ip link show master "$BRIDGE_NAME" 2>&1 | sed 's/^/  /'
    printf "\n"

    printf "${YELLOW}ARP Table:${NC}\n"
    ip neigh show dev "$BRIDGE_NAME" 2>&1 | sed 's/^/  /'
    if [ -z "$(ip neigh show dev "$BRIDGE_NAME" 2>/dev/null)" ]; then
        printf "  (empty - VM may not have sent any traffic)\n"
    fi
    printf "\n"

    printf "${YELLOW}IP Forwarding:${NC}\n"
    printf "  net.ipv4.ip_forward = %s\n" "$(cat /proc/sys/net/ipv4/ip_forward)"
    printf "\n"

    printf "${YELLOW}Suggested Diagnostic Commands:${NC}\n"
    printf "  sudo tcpdump -i %s -n              # Capture traffic on TAP\n" "$tap_name"
    printf "  sudo tcpdump -i %s -n icmp         # Capture only ICMP\n" "$tap_name"
    printf "  sudo arping -I %s %s               # Force ARP request\n" "$BRIDGE_NAME" "$ip"
    printf "  ip link show master %s             # Show bridge members\n" "$BRIDGE_NAME"
    printf "  bridge fdb show dev %s             # Show MAC forwarding table\n" "$tap_name"
    printf "\n"

    printf "${YELLOW}Log Files:${NC}\n"
    printf "  VM stdout:       %s/node-%d.log\n" "$VM_DIR" "$node_id"
    printf "  Serial console:  /tmp/aspen-node-%d-serial.log\n" "$node_id"
    printf "  Build log:       %s/build-%d.log\n" "$VM_DIR" "$node_id"
    printf "  virtiofsd log:   %s/virtiofsd-%d.log\n" "$VM_DIR" "$node_id"
    printf "\n"
}

# Show serial console output on VM failure
# Usage: show_serial_on_failure <node_id>
show_serial_on_failure() {
    local node_id="$1"
    local serial_log="/tmp/aspen-node-${node_id}-serial.log"

    if [ -f "$serial_log" ]; then
        printf "\n${YELLOW}=== Serial Console Output (last 30 lines) ===${NC}\n"
        tail -30 "$serial_log"
        printf "${YELLOW}=== End Serial Console ===${NC}\n"
        printf "Full serial log: %s\n" "$serial_log"
    else
        printf "\n${YELLOW}Serial console log not found: %s${NC}\n" "$serial_log"
        printf "The VM may not have started kernel boot yet.\n"
    fi
}
