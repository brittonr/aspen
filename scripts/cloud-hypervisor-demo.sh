#!/usr/bin/env bash
# Cloud Hypervisor Quick Demo for Aspen Testing
set -e

echo "Cloud Hypervisor Integration Demo"
echo "================================="
echo ""

# Check if we're in nix develop
if [ -z "$IN_NIX_SHELL" ]; then
    echo "Please run this script inside 'nix develop'"
    exit 1
fi

# Check Cloud Hypervisor availability
echo "1. Checking Cloud Hypervisor installation..."
if command -v cloud-hypervisor &> /dev/null; then
    echo "   ✓ Cloud Hypervisor found: $(cloud-hypervisor --version 2>&1 | head -1)"
else
    echo "   ✗ Cloud Hypervisor not found. Run 'nix develop' first."
    exit 1
fi

# Check KVM availability
echo ""
echo "2. Checking KVM support..."
if [ -e /dev/kvm ]; then
    echo "   ✓ KVM available"
else
    echo "   ✗ KVM not available. Please enable virtualization in BIOS."
    echo "   On NixOS, add: virtualisation.libvirtd.enable = true;"
    exit 1
fi

echo ""
echo "3. Available helper commands:"
echo "   • aspen-vm-setup  - Set up network bridges and TAP devices"
echo "   • aspen-vm-run    - Launch a test VM"
echo ""

echo "4. Quick start examples:"
echo ""
echo "   # Set up the test environment (requires sudo)"
echo "   $ aspen-vm-setup"
echo ""
echo "   # Launch Cloud Hypervisor API server"
echo "   $ cloud-hypervisor --api-socket /tmp/ch.sock"
echo ""
echo "   # In another terminal, create a VM via API"
echo "   $ curl --unix-socket /tmp/ch.sock -X PUT http://localhost/api/v1/vm.create \\"
echo "     -H 'Content-Type: application/json' \\"
echo "     -d '{\"cpus\": {\"boot_vcpus\": 2}, \"memory\": {\"size\": 536870912}}'"
echo ""
echo "   # Or launch a VM directly"
echo "   $ aspen-vm-run 1  # Launches VM with node ID 1"
echo ""

echo "5. Environment variables available:"
echo "   • CH_KERNEL=$CH_KERNEL"
echo "   • CH_FIRMWARE=$CH_FIRMWARE"
echo ""

echo "6. Build custom Cloud Hypervisor from vendored source:"
echo "   $ nix build .#cloud-hypervisor-custom"
echo ""

echo "7. Run VM-based integration tests:"
echo "   $ sudo cargo nextest run test_vm_"
echo ""

echo "Ready to test with Cloud Hypervisor!"