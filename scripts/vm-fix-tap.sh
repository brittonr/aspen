#!/usr/bin/env bash
# Fix for TAP device issues with Cloud Hypervisor
set -e

echo "Cloud Hypervisor TAP Device Fix"
echo "================================"
echo ""
echo "The 'tap already exists' error occurs because Cloud Hypervisor"
echo "tries to create a new TAP device when you specify --net tap=name."
echo ""
echo "Solutions:"
echo ""

echo "1. Use Cloud Hypervisor's auto-generated TAP (simplest):"
echo "   sudo cloud-hypervisor \\"
echo "     --kernel \$CH_KERNEL \\"
echo "     --disk path=/tmp/disk.img \\"
echo "     --cmdline \"console=hvc0\" \\"
echo "     --cpus boot=2 \\"
echo "     --memory size=512M \\"
echo "     --net \"mac=52:54:00:00:00:01\" \\"
echo "     --serial tty"
echo ""

echo "2. Remove existing TAP and let CH create it:"
echo "   sudo ip link del aspen-tap0"
echo "   # Then run aspen-vm-run"
echo ""

echo "3. Use a unique TAP name per VM:"
NODE_ID=${1:-1}
UNIQUE_TAP="vm${NODE_ID}tap"
echo "   sudo cloud-hypervisor \\"
echo "     --kernel \$CH_KERNEL \\"
echo "     --disk path=/tmp/disk.img \\"
echo "     --net \"tap=$UNIQUE_TAP,mac=52:54:00:00:00:0$NODE_ID\" \\"
echo "     --serial tty"
echo ""

echo "4. Run VM without TAP network (for testing):"
./scripts/vm-quick-start.sh "$NODE_ID"