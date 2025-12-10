# Cloud Hypervisor: Quick Reference for Aspen Integration

## TL;DR

Cloud Hypervisor is a lightweight Rust-based VMM perfect for spawning isolated test VMs for Aspen's distributed systems testing. It offers:

- **100-200ms boot time** (direct kernel boot)
- **50-200 VMs per host** (high density)
- **REST API** for programmatic control
- **Network/CPU/memory isolation** per VM
- **Minimal overhead** (~110MB per 4vCPU/1GB RAM VM)

---

## Architecture Overview

```
Aspen Test Orchestrator
    ↓
cloud-hypervisor (REST API) ← /tmp/ch.sock
    ├─ VM Node 1 (2 vCPU, 512MB, TAP net, VSOCK)
    ├─ VM Node 2 (2 vCPU, 512MB, TAP net, VSOCK)
    ├─ VM Node 3 (2 vCPU, 512MB, TAP net, VSOCK)
    └─ ... more VMs
```

---

## Quick Start: Creating a Test VM

### 1. Start Cloud Hypervisor (API server only)

```bash
./cloud-hypervisor --api-socket /tmp/ch.sock
```

### 2. Create VM via REST API

```bash
curl --unix-socket /tmp/ch.sock -X PUT http://localhost/api/v1/vm.create \
  -H "Content-Type: application/json" \
  -d '{
    "cpus": {"boot_vcpus": 2, "max_vcpus": 4},
    "memory": {"size": 536870912},
    "disks": [{"path": "/tmp/node.raw"}],
    "kernel": "/path/to/vmlinux",
    "cmdline": "root=/dev/vda1 rw console=hvc0",
    "net": [{"ip": "192.168.1.100", "mask": "255.255.255.0", "mac": "12:34:56:78:90:01"}],
    "vsock": {"cid": 3, "socket": "/tmp/node.vsock"}
  }'
```

### 3. Boot VM

```bash
curl --unix-socket /tmp/ch.sock -X PUT http://localhost/api/v1/vm.boot
```

### 4. Query Status

```bash
curl --unix-socket /tmp/ch.sock -X GET http://localhost/api/v1/vm.info
```

### 5. Manage Resources at Runtime

```bash
# Add CPU/Memory
curl --unix-socket /tmp/ch.sock -X PUT http://localhost/api/v1/vm.resize \
  -d '{"cpu_count": 4}'

# Add network device
curl --unix-socket /tmp/ch.sock -X PUT http://localhost/api/v1/vm.add-net \
  -d '{"ip": "192.168.1.101", "mask": "255.255.255.0", "mac": "12:34:56:78:90:02"}'

# Add disk
curl --unix-socket /tmp/ch.sock -X PUT http://localhost/api/v1/vm.add-disk \
  -d '{"path": "/tmp/extra.raw"}'
```

### 6. Shutdown

```bash
# Graceful
curl --unix-socket /tmp/ch.sock -X PUT http://localhost/api/v1/vm.shutdown

# Or delete
curl --unix-socket /tmp/ch.sock -X DELETE http://localhost/api/v1/vm.delete
```

---

## Key Configuration Options

### CPU

```rust
CpusConfig {
    boot_vcpus: u8,           // 1-255, vCPUs at boot
    max_vcpus: u8,            // ≥ boot_vcpus
    topology: (threads, cores, dies, packages),
    affinity: vec![CpuAffinity { vcpu: 0, host_cpus: [0,1,2,3] }],
    nested: bool,             // nested virtualization
}
```

### Memory

```rust
MemoryConfig {
    size: u64,                // 512MB-256GB typical
    shared: bool,             // for vhost devices
    hugepages: bool,          // 2M/1G pages
    mergeable: bool,          // KSM eligible
    thp: bool,                // transparent huge pages
    hotplug_size: u64,        // for dynamic resize
    zones: vec![...],         // NUMA zones
}
```

### Storage

```rust
DiskConfig {
    path: String,             // RAW, QCOW2, VHD, VHDX
    readonly: bool,
    direct: bool,             // bypass page cache
    num_queues: usize,        // parallelism
    rate_limiter: Option<...>,
}
```

### Network

```rust
NetConfig {
    tap: String,              // host TAP device (auto-created)
    ip: IpAddr,               // guest IP
    mask: IpAddr,             // netmask
    mac: MacAddr,             // MAC address
    num_queues: usize,        // virtio queues
    vhost_user: bool,         // offloaded backend
}
```

### Filesystem Sharing

```rust
FsConfig {
    socket: String,           // virtiofsd socket path
    tag: String,              // mount tag
    num_queues: usize,
    cache: CachePolicy,       // never, always
}
```

### VSOCK (Guest-Host Communication)

```rust
VsockConfig {
    cid: u32,                 // context ID (3 for guest)
    socket: String,           // unix socket path
}
```

---

## Performance Targets

| Metric | Value |
| ------ | ----- |
| VM Boot Time | 100-200ms (direct kernel) |
| VMM Overhead | ~110MB per (4vCPU, 1GB RAM) |
| Network Throughput | >90% native |
| Storage I/O Latency | +10-50μs per I/O |
| VM Density | 50-200 per host |
| CPU Density | 4-10x physical cores |

---

## Aspen Integration Pattern

### Test Setup

```rust
// 1. Spawn 5-node cluster
for i in 0..5 {
    let config = VmConfig {
        cpus: CpusConfig { boot_vcpus: 2, max_vcpus: 4 },
        memory: MemoryConfig { size: 512 * 1024 * 1024 },
        disks: vec![...],
        net: vec![NetConfig { ip: "192.168.1.{}", mac: ... }],
        vsock: Some(VsockConfig { cid: 3, socket: "/tmp/node{}.vsock" }),
        ..Default::default()
    };

    // REST API: create + boot
    create_vm("/tmp/ch.sock", &config)?;
    boot_vm("/tmp/ch.sock")?;
}

// 2. Wait for boot
sleep(Duration::from_secs(5));

// 3. Run test workload
for node in 0..5 {
    // SSH or gRPC to node
    run_test_command(node, "aspen-cli write key=value")?;
}

// 4. Inject faults
// Block network on node 1
manipulate_tap("/tmp/ch-tap1", "block")?;
sleep(Duration::from_secs(30));

// 5. Verify recovery
assert_convergence(&cluster)?;

// 6. Cleanup (snapshots for post-mortem)
snapshot_cluster("/tmp/snapshots/test-run-001")?;
shutdown_all_vms()?;
```

---

## Network Isolation for Testing

### Per-VM Isolation

- Each VM gets dedicated TAP device (`ch-tap0`, `ch-tap1`, etc.)
- Full L2/L3 isolation via host network stack
- Can simulate network partitions by blocking TAP

### Fault Injection Examples

```bash
# Block TAP device (network partition)
ip link set ch-tap1 down
sleep 30
ip link set ch-tap1 up

# Limit bandwidth
tc qdisc add dev ch-tap1 root tbf rate 10mbit burst 32kbit latency 400ms

# Add latency
tc qdisc add dev ch-tap1 root netem delay 100ms

# Packet loss
tc qdisc add dev ch-tap1 root netem loss 10%
```

---

## Guest Image Requirements

### Minimal Linux Image

- **Size:** 300-500MB
- **Format:** RAW (fastest), QCOW2 (flexible)
- **Base:** Ubuntu Jammy, Debian Bookworm, CentOS Stream
- **Kernel:** 5.10+ (for virtio-fs, VSOCK)
- **Init:** systemd, init script, or custom
- **Root:** `/dev/vda1` (virtio-blk device)

### Cloud-Init (Optional)

- Auto-configure network, users, packages
- Reduces setup time for test infrastructure

### Custom Image

```bash
# 1. Start from cloud image
wget https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64.raw

# 2. Mount and customize
mkdir /tmp/mnt
qemu-nbd -c /dev/nbd0 jammy-server-cloudimg-amd64.raw
mount /dev/nbd0p1 /tmp/mnt
chroot /tmp/mnt apt install <packages>
umount /tmp/mnt
qemu-nbd -d /dev/nbd0

# 3. Or use Cloud Hypervisor's build script
./scripts/build-custom-image.sh
```

---

## Rust API Usage

### REST API Client

```rust
use api_client::simple_api_full_command;
use std::os::unix::net::UnixStream;

fn create_vm(socket: &mut UnixStream, config: &str) -> Result<()> {
    simple_api_full_command(socket, "PUT", "vm.create", Some(config), &[])?;
    Ok(())
}

fn boot_vm(socket: &mut UnixStream) -> Result<()> {
    simple_api_full_command(socket, "PUT", "vm.boot", None, &[])?;
    Ok(())
}
```

### Test Infrastructure

```rust
use test_infra::{Guest, GuestCommand, UbuntuDiskConfig};

let disk = UbuntuDiskConfig::new("jammy-server-cloudimg-amd64.raw".into());
let guest = Guest::new(disk, temp_dir);

let mut proc = GuestCommand::new(&guest)
    .args(&["--cpus", "boot=2"])
    .args(&["--memory", "size=512M"])
    .spawn()?;

guest.wait_vm_boot(None)?;
let cpu_count = guest.get_cpu_count()?;
guest.ssh_command("uname -a")?;
```

---

## Build & Deploy

### Prerequisites

```bash
# Rust toolchain
rustup install 1.75+

# Linux kernel headers, build tools
sudo apt install linux-headers-generic build-essential

# Optional: static build
rustup target add x86_64-unknown-linux-musl
```

### Build

```bash
# Standard release
cargo build --release

# Static musl (portable)
cargo build --release --target x86_64-unknown-linux-musl

# Enable capabilities
sudo setcap cap_net_admin+ep ./target/release/cloud-hypervisor
```

### Features

```bash
# D-Bus API
--features dbus_api

# SEV-SNP, TDX, IGVM support
--features sev_snp,tdx,igvm

# All optional features
--features dbus_api,fw_cfg,ivshmem,guest_debug,sev_snp,tdx,igvm
```

---

## Common Issues & Solutions

### TAP Device Creation Fails

- Ensure `CAP_NET_ADMIN` capability: `sudo setcap cap_net_admin+ep cloud-hypervisor`
- Or run with `sudo`

### VM Won't Boot

- Check kernel/disk paths exist
- Verify kernel matches guest architecture (vmlinux-x86_64 for x86_64)
- Use `--serial tty` or `--console on` to see boot messages

### Network Not Working

- Ensure guest IP in same subnet as TAP device
- Check `/proc/net/dev` for TAP device presence
- Ping from host to guest IP

### OOM (Out of Memory)

- Reduce per-VM memory: `--memory size=256M`
- Reduce max VM count
- Enable KSM: `--memory mergeable=on`

### Slow I/O

- Use RAW disk format instead of QCOW2
- Enable direct I/O: `disk direct=on`
- Increase queue depth: `num_queues=4`

---

## Further Reading

- **Full Analysis:** `/home/brittonr/git/aspen/cloud-hypervisor-analysis.md` (1100+ lines)
- **API Documentation:** `/home/brittonr/git/cloud-hypervisor/docs/api.md`
- **Networking:** `/home/brittonr/git/cloud-hypervisor/docs/macvtap-bridge.md`
- **Storage:** `/home/brittonr/git/cloud-hypervisor/docs/fs.md`
- **CPU/Memory:** `/home/brittonr/git/cloud-hypervisor/docs/cpu.md`, `memory.md`
- **VSOCK:** `/home/brittonr/git/cloud-hypervisor/docs/vsock.md`
- **Building:** `/home/brittonr/git/cloud-hypervisor/docs/building.md`
