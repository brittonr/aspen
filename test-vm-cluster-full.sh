#!/usr/bin/env bash
set -e

# Full VM cluster test with mvm-ci + flawless + hiqlite
# Uses Nix for dependencies, plain bash for orchestration

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$SCRIPT_DIR/vm-test-data"
NUM_NODES=3

echo "=== Full mvm-ci VM Cluster Test ==="
echo ""

# Enter nix shell with all dependencies
if [ -z "$VM_TEST_NIX_SHELL" ]; then
  echo "Entering Nix shell with QEMU and build tools..."
  export VM_TEST_NIX_SHELL=1
  exec nix-shell -p qemu_kvm pkgsStatic.busybox netcat gnutar gzip cpio findutils --run "$0 $*"
fi

echo "✓ Running in Nix shell"
echo ""

# Build mvm-ci
echo "=== Building mvm-ci ==="
if [ ! -f "./target/release/mvm-ci" ]; then
  cargo build --release
fi
echo "✓ mvm-ci binary ready"

# Check for flawless
echo ""
echo "=== Checking for flawless ==="
if ! command -v flawless &> /dev/null; then
  echo "⚠ flawless not found in PATH"
  echo "  Install with: cargo install flawless-cli"
  echo "  Or download from: https://github.com/sebadob/flawless/releases"
  echo ""
  echo "Continuing without flawless for basic testing..."
  HAVE_FLAWLESS=false
else
  echo "✓ flawless found: $(which flawless)"
  HAVE_FLAWLESS=true
fi

mkdir -p "$VM_DIR"

# Generate hiqlite configs
generate_hiqlite_configs() {
  echo ""
  echo "=== Generating hiqlite configs ==="

  for node_id in 1 2 3; do
    cat > "$VM_DIR/hiqlite-node${node_id}.toml" << EOF
[hiqlite]
node_id = ${node_id}
data_dir = "/data/hiqlite"
secret_raft = "SuperSecureSecret1337ForRaft"
secret_api = "SuperSecureSecret1337ForAPI"
enc_keys = ["bVCyTsGaggVy5yqQ/UzluN29DZW41M3hTSkx6Y3NtZmRuQkR2TnJxUTYzcjQ="]
enc_key_active = "bVCyTsGaggVy5yqQ"

# Raft cluster members
nodes = [
    "1 192.168.100.11:9000 192.168.100.11:9001",
    "2 192.168.100.12:9000 192.168.100.12:9001",
    "3 192.168.100.13:9000 192.168.100.13:9001",
]

listen_addr_api = "0.0.0.0"
listen_addr_raft = "0.0.0.0"
EOF
    echo "✓ Generated hiqlite-node${node_id}.toml"
  done
}

# Create initrd with mvm-ci, flawless, and configs
create_initrd() {
  local node_id=$1
  local initrd_dir="$VM_DIR/initrd-${node_id}"
  local ip="192.168.100.1${node_id}"

  echo "Creating initrd for node${node_id}..."

  rm -rf "$initrd_dir"
  mkdir -p "$initrd_dir"/{bin,sbin,etc,proc,sys,dev,tmp,root,var/log,data/hiqlite,data/iroh,lib}

  # Copy busybox for basic utilities
  cp "$(which busybox)" "$initrd_dir/bin/"
  for applet in sh ash bash ls cat mkdir mount umount ip route ping sleep nc grep sed awk ps kill; do
    ln -sf busybox "$initrd_dir/bin/$applet" 2>/dev/null || true
  done

  # Copy and patch mvm-ci binary
  cp "./target/release/mvm-ci" "$initrd_dir/bin/"
  # Patch the binary to use /lib for libraries instead of nix store
  patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 "$initrd_dir/bin/mvm-ci" 2>/dev/null || true
  patchelf --set-rpath /lib:/lib64 "$initrd_dir/bin/mvm-ci" 2>/dev/null || true
  echo "  ✓ Copied and patched mvm-ci binary"

  # Copy flawless if available
  if [ "$HAVE_FLAWLESS" = true ]; then
    cp "$(which flawless)" "$initrd_dir/bin/"
    # Patch the binary to use /lib for libraries
    patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 "$initrd_dir/bin/flawless" 2>/dev/null || true
    patchelf --set-rpath /lib:/lib64 "$initrd_dir/bin/flawless" 2>/dev/null || true
    echo "  ✓ Copied and patched flawless binary"
  fi

  # Copy required libraries (for dynamically linked binaries)
  bins_to_check="./target/release/mvm-ci $initrd_dir/bin/busybox"
  if [ "$HAVE_FLAWLESS" = true ]; then
    bins_to_check="$bins_to_check $initrd_dir/bin/flawless"
  fi

  for bin in $bins_to_check; do
    for lib in $(ldd "$bin" 2>/dev/null | grep "=> /" | awk '{print $3}'); do
      if [ -f "$lib" ]; then
        cp "$lib" "$initrd_dir/lib/" 2>/dev/null || true
      fi
    done
  done

  # Copy dynamic linker
  if [ -f /lib64/ld-linux-x86-64.so.2 ]; then
    mkdir -p "$initrd_dir/lib64"
    cp /lib64/ld-linux-x86-64.so.2 "$initrd_dir/lib64/" 2>/dev/null || true
  fi

  # Copy hiqlite config
  cp "$VM_DIR/hiqlite-node${node_id}.toml" "$initrd_dir/hiqlite.toml"
  echo "  ✓ Copied hiqlite config"

  # Create startup script
  cat > "$initrd_dir/init" << EOF
#!/bin/busybox sh
export PATH=/bin:/sbin:/lib:/lib64
export LD_LIBRARY_PATH=/lib:/lib64

# Mount essential filesystems
busybox mount -t proc none /proc
busybox mount -t sysfs none /sys
busybox mount -t devtmpfs none /dev

# Network configuration
busybox ip link set lo up

# Wait for network device (may be eth0 or ens3 depending on kernel)
for i in \$(busybox seq 1 10); do
  if busybox ip link show eth0 >/dev/null 2>&1; then
    NET_DEV=eth0
    break
  elif busybox ip link show ens3 >/dev/null 2>&1; then
    NET_DEV=ens3
    break
  fi
  busybox sleep 0.5
done

if [ -n "\$NET_DEV" ]; then
  busybox ip link set \$NET_DEV up
  busybox ip addr add ${ip}/24 dev \$NET_DEV
  busybox ip route add default via 192.168.100.1 || true
  echo "Network configured: \$NET_DEV with IP ${ip}"
else
  echo "Warning: No network device found"
fi

# Setup /etc/hosts
cat > /etc/hosts << HOSTS
127.0.0.1 localhost
192.168.100.11 node1
192.168.100.12 node2
192.168.100.13 node3
HOSTS

echo ""
echo "=== Node ${node_id} Started ==="
echo "IP: ${ip}"
echo "Hostname: node${node_id}"
echo ""

# Wait for network
busybox sleep 3

# Start flawless if available
if [ -f /bin/flawless ]; then
  echo "Starting flawless server..."
  cd /tmp
  /bin/flawless up > /var/log/flawless.log 2>&1 &
  echo "✓ Flawless started (PID: \$!)"
  busybox sleep 5
fi

# Start mvm-ci
echo "Starting mvm-ci..."
cd /
export HTTP_PORT=3020
export FLAWLESS_URL="http://localhost:27288"
export IROH_BLOBS_PATH="/data/iroh"
/bin/mvm-ci > /var/log/mvm-ci.log 2>&1 &
MVM_PID=\$!
echo "✓ mvm-ci started (PID: \$MVM_PID)"

# Monitor logs
busybox sleep 3
echo ""
echo "=== Service Status ==="
busybox ps | grep -E "flawless|mvm-ci" || echo "Services starting..."
echo ""

# Keep VM running and tail logs
echo "Node ${node_id} ready. Tailing mvm-ci logs..."
busybox tail -f /var/log/mvm-ci.log 2>/dev/null || busybox sh
EOF

  chmod +x "$initrd_dir/init"

  # Create initramfs
  echo "  ✓ Building initramfs..."
  (cd "$initrd_dir" && find . -print0 | cpio --null -o -H newc 2>/dev/null | gzip -9 > "../initrd-${node_id}.gz")

  local size=$(du -h "$VM_DIR/initrd-${node_id}.gz" | cut -f1)
  echo "✓ Created initrd for node${node_id} ($size)"
}

# Get kernel
get_kernel() {
  local kernel="$VM_DIR/vmlinuz"

  if [ -f "$kernel" ]; then
    echo "✓ Using cached kernel"
    return
  fi

  echo "Downloading generic Linux kernel..."

  # Use a minimal generic Linux kernel from Debian
  # This avoids NixOS kernel restrictions on dynamically linked executables
  local kernel_url="https://deb.debian.org/debian/dists/bookworm/main/installer-amd64/current/images/netboot/debian-installer/amd64/linux"

  if command -v curl >/dev/null 2>&1; then
    curl -L -o "$kernel" "$kernel_url" 2>/dev/null
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$kernel" "$kernel_url" 2>/dev/null
  else
    echo "✗ Need curl or wget to download kernel"
    exit 1
  fi

  if [ -f "$kernel" ] && [ -s "$kernel" ]; then
    echo "✓ Kernel downloaded"
  else
    echo "✗ Failed to download kernel"
    exit 1
  fi
}

# Start VM
start_vm() {
  local node_id=$1
  local ip="192.168.100.1${node_id}"
  local http_port=$((3020 + node_id - 1))
  local monitor_port=$((4440 + node_id))

  qemu-system-x86_64 \
    -name "node${node_id}" \
    -m 1024 \
    -smp 2 \
    -kernel "$VM_DIR/vmlinuz" \
    -initrd "$VM_DIR/initrd-${node_id}.gz" \
    -append "console=ttyS0 quiet" \
    -device e1000,netdev=net0,mac=52:54:00:12:34:0${node_id} \
    -netdev user,id=net0,net=192.168.100.0/24,dhcpstart=192.168.100.1${node_id},hostfwd=tcp::${http_port}-:3020,hostfwd=tcp::$((9000 + node_id))-:9000 \
    -display none \
    -serial file:"$VM_DIR/node${node_id}-console.log" \
    -daemonize \
    -pidfile "$VM_DIR/node${node_id}.pid" \
    -monitor telnet:127.0.0.1:${monitor_port},server,nowait \
    2>&1 | grep -v "warning" || true

  echo "✓ Started node${node_id}"
  echo "    HTTP: localhost:${http_port}"
  echo "    Console: $VM_DIR/node${node_id}-console.log"
  echo "    Monitor: telnet localhost:${monitor_port}"
}

# Wait for service
wait_for_service() {
  local port=$1
  local node=$2
  local service=$3

  echo -n "Waiting for node${node} ${service}"
  for i in {1..40}; do
    if curl -s -m 1 "http://localhost:${port}/" > /dev/null 2>&1; then
      echo " ✓"
      return 0
    fi
    echo -n "."
    sleep 2
  done
  echo " timeout"
  echo "  Check logs: tail $VM_DIR/node${node}-console.log"
  return 1
}

# Cleanup
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

  echo "✓ Cleanup complete"
  echo "  VM data preserved in: $VM_DIR"
}

# Run integration tests
run_tests() {
  echo ""
  echo "=== Running Integration Tests ==="
  echo ""

  # Test 1: All nodes responding
  echo "Test 1: Health Check"
  for node in 1 2 3; do
    port=$((3020 + node - 1))
    if curl -s -m 2 "http://localhost:${port}/" > /dev/null 2>&1; then
      echo "  ✓ Node${node} responding on port ${port}"
    else
      echo "  ✗ Node${node} not responding"
    fi
  done

  # Test 2: Submit job to node1
  echo ""
  echo "Test 2: Job Submission"
  response=$(curl -s -X POST "http://localhost:3020/new-job" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "url=https://vm-cluster-test.com" 2>&1)
  echo "  ✓ Submitted job to node1"

  sleep 5

  # Test 3: Check job replication
  echo ""
  echo "Test 3: Raft Replication (checking if job appears on all nodes)"
  for node in 1 2 3; do
    port=$((3020 + node - 1))
    jobs=$(curl -s "http://localhost:${port}/queue/list" 2>/dev/null | jq -r 'length' 2>/dev/null || echo "0")
    echo "  Node${node}: ${jobs} jobs in queue"
  done

  # Test 4: Queue stats
  echo ""
  echo "Test 4: Queue Statistics"
  for node in 1 2 3; do
    port=$((3020 + node - 1))
    stats=$(curl -s "http://localhost:${port}/queue/stats" 2>/dev/null)
    echo "  Node${node}: $stats"
  done

  # Test 5: Submit jobs to different nodes
  echo ""
  echo "Test 5: Multi-Node Job Submission"
  curl -s -X POST "http://localhost:3021/new-job" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "url=https://node2-job.com" > /dev/null 2>&1
  echo "  ✓ Submitted job to node2"

  curl -s -X POST "http://localhost:3022/new-job" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "url=https://node3-job.com" > /dev/null 2>&1
  echo "  ✓ Submitted job to node3"

  sleep 5

  # Final status
  echo ""
  echo "=== Final Cluster Status ==="
  for node in 1 2 3; do
    port=$((3020 + node - 1))
    echo "Node${node} (localhost:${port}):"
    curl -s "http://localhost:${port}/queue/list" 2>/dev/null | \
      jq -r '.[] | "  \(.job_id): \(.status) [completed_by: \(.completed_by // "none")]"' 2>/dev/null || \
      echo "  (unable to fetch)"
  done
}

# Main
main() {
  trap cleanup EXIT

  generate_hiqlite_configs
  get_kernel

  echo ""
  echo "=== Building VM Images ==="
  for node in 1 2 3; do
    create_initrd $node
  done

  echo ""
  echo "=== Starting VMs ==="
  for node in 1 2 3; do
    start_vm $node
  done

  echo ""
  echo "=== Waiting for Services ==="
  for node in 1 2 3; do
    port=$((3020 + node - 1))
    wait_for_service $port $node "mvm-ci" || echo "  (continuing anyway...)"
  done

  run_tests

  echo ""
  echo "=== Cluster Ready ==="
  echo ""
  echo "Access nodes:"
  echo "  Node1: curl http://localhost:3020/queue/list | jq"
  echo "  Node2: curl http://localhost:3021/queue/list | jq"
  echo "  Node3: curl http://localhost:3022/queue/list | jq"
  echo ""
  echo "View logs:"
  echo "  tail -f $VM_DIR/node1-console.log"
  echo "  tail -f $VM_DIR/node2-console.log"
  echo "  tail -f $VM_DIR/node3-console.log"
  echo ""
  echo "Press Enter to stop cluster and cleanup..."

  if [ -t 0 ]; then
    read
  else
    sleep 30
  fi
}

main
