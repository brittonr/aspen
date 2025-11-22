#!/usr/bin/env bash
set -e

# Simple VM-based cluster testing using QEMU and cloud-init
# Much simpler than NixOS tests!

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$SCRIPT_DIR/vm-test-data"
IMAGE_URL="https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-generic-amd64.qcow2"
IMAGE_NAME="debian-12-generic-amd64.qcow2"

echo "=== VM Cluster Test Setup ==="
echo ""

# Check dependencies
check_deps() {
  local missing=()
  for cmd in qemu-system-x86_64 qemu-img cloud-localds; do
    if ! command -v $cmd &> /dev/null; then
      missing+=($cmd)
    fi
  done

  if [ ${#missing[@]} -ne 0 ]; then
    echo "Missing dependencies: ${missing[*]}"
    echo ""
    echo "Install with:"
    echo "  Ubuntu/Debian: sudo apt-get install qemu-system qemu-utils cloud-image-utils"
    echo "  Fedora: sudo dnf install qemu-system qemu-img cloud-utils"
    echo "  Arch: sudo pacman -S qemu-full cloud-utils"
    echo "  macOS: brew install qemu"
    exit 1
  fi
}

# Download base image
download_image() {
  mkdir -p "$VM_DIR"

  if [ ! -f "$VM_DIR/$IMAGE_NAME" ]; then
    echo "Downloading Debian cloud image..."
    curl -L "$IMAGE_URL" -o "$VM_DIR/$IMAGE_NAME"
    echo "✓ Image downloaded"
  else
    echo "✓ Using cached image"
  fi
}

# Create VM disk from base image
create_vm_disk() {
  local node_id=$1
  local disk="$VM_DIR/node${node_id}.qcow2"

  if [ -f "$disk" ]; then
    rm "$disk"
  fi

  qemu-img create -f qcow2 -F qcow2 -b "$VM_DIR/$IMAGE_NAME" "$disk" 10G > /dev/null
  echo "✓ Created disk for node${node_id}"
}

# Create cloud-init config for a node
create_cloudinit() {
  local node_id=$1
  local ssh_port=$2

  cat > "$VM_DIR/user-data-node${node_id}.yaml" <<EOF
#cloud-config
hostname: node${node_id}
fqdn: node${node_id}.local

users:
  - name: test
    sudo: ALL=(ALL) NOPASSWD:ALL
    shell: /bin/bash
    ssh_authorized_keys:
      - ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC... # Your SSH key here

packages:
  - curl
  - jq
  - iptables
  - python3
  - build-essential

write_files:
  - path: /etc/hosts
    content: |
      127.0.0.1 localhost
      192.168.100.10 node1
      192.168.100.11 node2
      192.168.100.12 node3

runcmd:
  - echo "Node ${node_id} initialized" > /tmp/init-done
EOF

  cat > "$VM_DIR/meta-data-node${node_id}.yaml" <<EOF
instance-id: node${node_id}
local-hostname: node${node_id}
EOF

  cloud-localds "$VM_DIR/node${node_id}-cidata.iso" \
    "$VM_DIR/user-data-node${node_id}.yaml" \
    "$VM_DIR/meta-data-node${node_id}.yaml" 2>/dev/null

  echo "✓ Created cloud-init for node${node_id}"
}

# Start a VM node
start_node() {
  local node_id=$1
  local ssh_port=$2
  local ip="192.168.100.1${node_id}"

  qemu-system-x86_64 \
    -name "node${node_id}" \
    -machine accel=kvm \
    -cpu host \
    -m 2048 \
    -smp 2 \
    -drive file="$VM_DIR/node${node_id}.qcow2",format=qcow2 \
    -drive file="$VM_DIR/node${node_id}-cidata.iso",format=raw \
    -device e1000,netdev=net0 \
    -netdev user,id=net0,hostfwd=tcp::${ssh_port}-:22 \
    -nographic \
    -daemonize \
    -pidfile "$VM_DIR/node${node_id}.pid" \
    2>&1 | grep -v "warning: TCG"

  echo "✓ Started node${node_id} (SSH port: ${ssh_port})"
}

# Wait for VM to boot
wait_for_vm() {
  local ssh_port=$1
  local node_name=$2

  echo -n "Waiting for $node_name to boot"
  for i in {1..30}; do
    if timeout 1 bash -c "echo > /dev/tcp/localhost/$ssh_port" 2>/dev/null; then
      echo " ✓"
      return 0
    fi
    echo -n "."
    sleep 2
  done
  echo " ✗ timeout"
  return 1
}

# SSH into a VM
vm_ssh() {
  local ssh_port=$1
  shift
  ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
      -p $ssh_port test@localhost "$@" 2>/dev/null
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
        echo "✓ Killed VM (PID: $pid)"
      fi
      rm "$pid_file"
    fi
  done

  # Clean up temp files but keep the base image
  rm -f "$VM_DIR"/node*.qcow2 "$VM_DIR"/*-cidata.iso "$VM_DIR"/*.yaml
}

# Run tests
run_tests() {
  echo ""
  echo "=== Running Tests ==="
  echo ""

  # Test 1: Cross-VM connectivity
  echo "Test 1: Cross-VM Communication"
  vm_ssh 2222 "curl -s http://node2:8000/ --max-time 5" && echo "  ✓ node1 can reach node2" || echo "  ✗ Failed"
  vm_ssh 2223 "curl -s http://node3:8000/ --max-time 5" && echo "  ✓ node2 can reach node3" || echo "  ✗ Failed"

  # Test 2: Network partition
  echo ""
  echo "Test 2: Network Partition"
  vm_ssh 2222 "sudo iptables -A OUTPUT -d node2 -j DROP"
  vm_ssh 2222 "curl -s http://node2:8000/ --max-time 2" && echo "  ✗ Partition didn't work" || echo "  ✓ node1 isolated from node2"

  # Heal partition
  vm_ssh 2222 "sudo iptables -F"
  vm_ssh 2222 "curl -s http://node2:8000/ --max-time 5" && echo "  ✓ Partition healed" || echo "  ✗ Failed to heal"

  echo ""
  echo "=== Test Summary ==="
  echo "This is a simplified test. To run full mvm-ci cluster:"
  echo "  1. Install mvm-ci in each VM: vm_ssh 2222 'cargo install --path /path/to/mvm-ci'"
  echo "  2. Copy hiqlite configs to each VM"
  echo "  3. Start services: vm_ssh 2222 'systemctl start mvm-ci'"
  echo "  4. Run integration tests"
}

# Main
main() {
  trap cleanup EXIT

  check_deps
  download_image

  echo ""
  echo "=== Creating VMs ==="
  for i in 0 1 2; do
    node_id=$i
    ssh_port=$((2222 + i))
    create_vm_disk $node_id
    create_cloudinit $node_id $ssh_port
    start_node $node_id $ssh_port
  done

  echo ""
  echo "=== Waiting for VMs to boot ==="
  wait_for_vm 2222 "node0"
  wait_for_vm 2223 "node1"
  wait_for_vm 2224 "node2"

  # Start simple HTTP servers for testing
  for port in 2222 2223 2224; do
    vm_ssh $port "python3 -m http.server 8000 >/dev/null 2>&1 &" || true
  done

  sleep 3

  run_tests

  echo ""
  echo "To interact with VMs:"
  echo "  ssh -p 2222 test@localhost  # node0"
  echo "  ssh -p 2223 test@localhost  # node1"
  echo "  ssh -p 2224 test@localhost  # node2"
  echo ""
  echo "Press Enter to cleanup and exit..."
  read
}

# Check if running in interactive mode
if [ -t 0 ]; then
  main
else
  echo "Run with: ./test-vm-cluster.sh"
  exit 1
fi
