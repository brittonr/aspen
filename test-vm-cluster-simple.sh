#!/usr/bin/env bash
set -e

# Simple VM cluster testing using QEMU
# Uses Nix for dependencies, but plain bash for everything else

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$SCRIPT_DIR/vm-test-data"
NUM_NODES=3

echo "=== Simple VM Cluster Test ==="
echo ""

# Enter nix shell with all dependencies
if [ -z "$IN_NIX_SHELL" ]; then
  echo "Entering Nix shell with QEMU..."
  exec nix-shell -p qemu_kvm socat busybox netcat --run "$0 $*"
fi

echo "✓ Running in Nix shell with QEMU available"
echo ""

# Build mvm-ci if needed
if [ ! -f "./target/release/mvm-ci" ]; then
  echo "Building mvm-ci..."
  cargo build --release
fi

mkdir -p "$VM_DIR"

# Create a tiny Linux VM using busybox
create_initrd() {
  local node_id=$1
  local initrd_dir="$VM_DIR/initrd-${node_id}"

  rm -rf "$initrd_dir"
  mkdir -p "$initrd_dir"/{bin,sbin,etc,proc,sys,dev,tmp,root}

  # Copy busybox
  cp "$(which busybox)" "$initrd_dir/bin/"

  # Create symlinks for all busybox applets
  for applet in sh ash bash ls cat mkdir mount umount wget curl ip route ping sleep nc grep sed awk; do
    ln -sf busybox "$initrd_dir/bin/$applet" 2>/dev/null || true
  done

  # Create init script
  cat > "$initrd_dir/init" << 'EOF'
#!/bin/busybox sh
export PATH=/bin:/sbin

busybox --install -s

mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev

# Configure network based on kernel command line
for param in $(cat /proc/cmdline); do
  case $param in
    ip=*) IP=${param#ip=} ;;
    node=*) NODE=${param#node=} ;;
  esac
done

ip link set lo up
ip link set eth0 up
ip addr add $IP/24 dev eth0
ip route add default via 192.168.100.1

echo "Node $NODE booted with IP $IP"

# Add /etc/hosts
cat > /etc/hosts << HOSTS
192.168.100.10 node1
192.168.100.11 node2
192.168.100.12 node3
HOSTS

# Start a simple HTTP server for testing
mkdir -p /var/www
echo "Hello from $NODE" > /var/www/index.html
busybox httpd -p 8000 -h /var/www &

# Keep running
exec sh
EOF

  chmod +x "$initrd_dir/init"

  # Create initramfs
  (cd "$initrd_dir" && find . | cpio -o -H newc 2>/dev/null | gzip > "../initrd-${node_id}.gz")

  echo "✓ Created initrd for node${node_id}"
}

# Download a minimal kernel
get_kernel() {
  local kernel="$VM_DIR/vmlinuz"

  if [ ! -f "$kernel" ]; then
    echo "Extracting kernel from host..."
    # Use the host kernel
    if [ -f /boot/vmlinuz-$(uname -r) ]; then
      cp /boot/vmlinuz-$(uname -r) "$kernel"
    elif [ -f /run/current-system/kernel ]; then
      # NixOS
      cp /run/current-system/kernel "$kernel"
    else
      echo "Could not find kernel. Install kernel or use a cloud image."
      exit 1
    fi
    echo "✓ Kernel ready"
  fi
}

# Start a VM
start_vm() {
  local node_id=$1
  local ip="192.168.100.1${node_id}"
  local ssh_port=$((2220 + node_id))
  local monitor_port=$((4440 + node_id))

  qemu-system-x86_64 \
    -name "node${node_id}" \
    -m 512 \
    -smp 1 \
    -kernel "$VM_DIR/vmlinuz" \
    -initrd "$VM_DIR/initrd-${node_id}.gz" \
    -append "console=ttyS0 ip=${ip} node=${node_id}" \
    -device e1000,netdev=net0 \
    -netdev user,id=net0,hostfwd=tcp::${ssh_port}-:22,hostfwd=tcp::$((8000 + node_id))-:8000 \
    -nographic \
    -serial mon:stdio \
    -daemonize \
    -pidfile "$VM_DIR/node${node_id}.pid" \
    -monitor telnet:127.0.0.1:${monitor_port},server,nowait \
    2>&1 | grep -v "warning" || true

  echo "✓ Started node${node_id} (HTTP: localhost:$((8000 + node_id)), Monitor: telnet localhost:${monitor_port})"
}

# Wait for VM to respond
wait_vm() {
  local port=$1
  local node=$2

  echo -n "Waiting for node${node}"
  for i in {1..20}; do
    if curl -s -m 1 "http://localhost:${port}/" > /dev/null 2>&1; then
      echo " ✓"
      return 0
    fi
    echo -n "."
    sleep 1
  done
  echo " timeout"
  return 1
}

# Send command to VM via monitor
vm_command() {
  local node=$1
  local cmd=$2
  local monitor_port=$((4440 + node))

  echo "$cmd" | nc -q 1 localhost $monitor_port 2>/dev/null | tail -1
}

# Stop all VMs
cleanup() {
  echo ""
  echo "=== Cleaning up ==="

  for pid_file in "$VM_DIR"/node*.pid; do
    if [ -f "$pid_file" ]; then
      pid=$(cat "$pid_file")
      if kill -0 $pid 2>/dev/null; then
        kill $pid
        echo "✓ Stopped VM (PID: $pid)"
      fi
      rm "$pid_file"
    fi
  done

  # rm -rf "$VM_DIR"
  echo "✓ VM data in: $VM_DIR"
}

# Run tests
run_tests() {
  echo ""
  echo "=== Running Tests ==="
  echo ""

  # Test 1: VMs are up
  echo "Test 1: VM Health Check"
  for i in 1 2 3; do
    port=$((8000 + i))
    response=$(curl -s "http://localhost:${port}/")
    echo "  Node${i}: $response"
  done

  echo ""
  echo "Test 2: Cross-VM Communication"
  echo "  (This would require SSH/serial access to run commands inside VMs)"
  echo "  In practice, you'd:"
  echo "    - Deploy mvm-ci binary to each VM"
  echo "    - Copy hiqlite configs"
  echo "    - Start services"
  echo "    - Submit jobs via HTTP API"

  echo ""
  echo "=== VMs are ready! ==="
  echo ""
  echo "Access VMs:"
  echo "  Node1 HTTP: curl http://localhost:8001/"
  echo "  Node2 HTTP: curl http://localhost:8002/"
  echo "  Node3 HTTP: curl http://localhost:8003/"
  echo ""
  echo "VM Monitors (for debugging):"
  echo "  Node1: telnet localhost 4441"
  echo "  Node2: telnet localhost 4442"
  echo "  Node3: telnet localhost 4443"
  echo ""
  echo "To deploy mvm-ci:"
  echo "  1. Copy binary into VM rootfs (rebuild initrd)"
  echo "  2. Or use a full OS image with SSH"
  echo ""
  echo "Press Enter to cleanup and exit..."
}

# Main
main() {
  trap cleanup EXIT

  echo "=== Setup ==="
  get_kernel

  echo ""
  echo "=== Creating VMs ==="
  for i in 1 2 3; do
    create_initrd $i
    start_vm $i
  done

  echo ""
  echo "=== Waiting for boot ==="
  for i in 1 2 3; do
    wait_vm $((8000 + i)) $i
  done

  run_tests

  if [ -t 0 ]; then
    read
  else
    echo "Running in non-interactive mode, exiting"
    sleep 5
  fi
}

main
