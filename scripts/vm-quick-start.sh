#!/usr/bin/env bash
# Quick VM start script that handles TAP devices properly
set -e

NODE_ID=${1:-1}

echo "Cloud Hypervisor Quick Start for Node $NODE_ID"
echo "=============================================="
echo ""

# Check for KVM
if [ ! -e /dev/kvm ]; then
    echo "ERROR: KVM not available. Please ensure KVM is enabled."
    exit 1
fi

# Check for Cloud Hypervisor
if ! command -v cloud-hypervisor &> /dev/null; then
    echo "ERROR: cloud-hypervisor not found. Run 'nix develop' first."
    exit 1
fi

# Kernel path
KERNEL=${CH_KERNEL:-$(ls /nix/store/*/bzImage 2>/dev/null | head -1)}
if [ -z "$KERNEL" ] || [ ! -f "$KERNEL" ]; then
    echo "ERROR: Kernel not found. Please set CH_KERNEL or run in nix develop."
    exit 1
fi

# Create a minimal disk image
DISK="/tmp/aspen-node-$NODE_ID.img"
if [ ! -f "$DISK" ]; then
    echo "Creating disk image at $DISK..."
    qemu-img create -f raw "$DISK" 1G
    # Optional: Create a minimal filesystem
    # sudo mkfs.ext4 "$DISK"
fi

echo "Configuration:"
echo "  Node ID: $NODE_ID"
echo "  Kernel: $KERNEL"
echo "  Disk: $DISK"
echo ""

# Option 1: Run without network (simplest)
echo "Option 1: Running VM without network (simplest)..."
echo "Command: cloud-hypervisor --kernel \"$KERNEL\" --disk path=\"$DISK\" --cmdline \"console=hvc0\" --cpus boot=2 --memory size=512M --serial tty --console off"
echo ""
echo "Press Enter to start VM without network, or Ctrl+C to cancel..."
read -r

cloud-hypervisor \
    --kernel "$KERNEL" \
    --disk path="$DISK" \
    --cmdline "console=hvc0 init=/bin/sh" \
    --cpus boot=2 \
    --memory size=512M \
    --serial tty \
    --console off