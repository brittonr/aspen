# Cloud Hypervisor Integration in Aspen

This document explains how Cloud Hypervisor has been integrated into the Aspen project for VM-based testing.

## Integration Overview

We've integrated Cloud Hypervisor in **three ways**:

### 1. **DevShell Integration (Recommended)**
Cloud Hypervisor is now part of the development environment:
- Automatically available when you run `nix develop`
- Includes helper tools (bridge-utils, iproute2, qemu-utils)
- Pre-configured environment variables

### 2. **Custom Build from Vendored Source**
We have a vendored Cloud Hypervisor in `cloud-hypervisor/`:
- Optimized build with specific features (KVM, io_uring)
- Build with: `nix build .#cloud-hypervisor-custom`
- Allows local modifications for testing needs

### 3. **Helper Scripts**
Two helper scripts are provided in the devShell:
- `aspen-vm-setup`: Sets up network bridges and TAP devices
- `aspen-vm-run <node-id>`: Launches a test VM quickly

## Quick Start

### Step 1: Enter Development Environment
```bash
nix develop
```

### Step 2: Check Cloud Hypervisor
```bash
cloud-hypervisor --version
# Output: cloud-hypervisor v48.0
```

### Step 3: Set Up Network (One-time, requires sudo)
```bash
aspen-vm-setup
```

### Step 4: Launch Cloud Hypervisor

**Option A: API Server Mode**
```bash
# Terminal 1: Start API server
cloud-hypervisor --api-socket /tmp/ch.sock

# Terminal 2: Create VM via API
curl --unix-socket /tmp/ch.sock -X PUT http://localhost/api/v1/vm.create \
  -H "Content-Type: application/json" \
  -d '{
    "cpus": {"boot_vcpus": 2},
    "memory": {"size": 536870912},
    "kernel": {"path": "'"$CH_KERNEL"'"},
    "cmdline": {"args": "console=hvc0"}
  }'

# Boot the VM
curl --unix-socket /tmp/ch.sock -X PUT http://localhost/api/v1/vm.boot
```

**Option B: Direct Launch**
```bash
aspen-vm-run 1  # Launches VM with node ID 1
```

## Environment Variables

The following are automatically set in the devShell:
- `CH_KERNEL`: Path to Linux kernel bzImage
- `CH_FIRMWARE`: Path to OVMF UEFI firmware
- `CH_API_SOCKET`: Default API socket path (/tmp/ch.sock)

## Testing Infrastructure

### VM Manager (`src/testing/vm_manager.rs`)
High-level API for managing Cloud Hypervisor VMs:
```rust
use aspen::testing::vm_manager::{VmManager, VmConfig};

let manager = VmManager::new()?;
let cluster = manager.launch_cluster(3).await?;
cluster.wait_for_health(Duration::from_secs(30)).await?;
```

### Network Utilities (`src/testing/network_utils.rs`)
Network bridge and TAP device management:
```rust
use aspen::testing::network_utils::{NetworkBridge, TapDevice};

let bridge = NetworkBridge::create("aspen-br0", "10.100.0.1/24")?;
let tap = TapDevice::create("aspen-tap0", Some(&bridge))?;
```

### Fault Injection (`src/testing/fault_injection.rs`)
Network partitions and latency injection:
```rust
use aspen::testing::fault_injection::{NetworkPartition, LatencyInjection};

let partition = NetworkPartition::create("10.100.0.2", &["10.100.0.3"])?;
// ... test with partition ...
partition.heal()?;
```

## Running VM Tests

### Unit Tests (Mock VMs)
```bash
cargo nextest run test_vm_
```

### Integration Tests (Real VMs, requires root)
```bash
sudo cargo nextest run test_vm_basic
```

## Architecture

```
Aspen Test Framework
        │
        ├─── VM Manager (Rust)
        │    ├─ Cloud Hypervisor REST API Client
        │    ├─ Network Configuration (Bridge/TAP)
        │    └─ Health Checking
        │
        ├─── Cloud Hypervisor (VMM)
        │    ├─ API Server (/tmp/ch.sock)
        │    ├─ VM Process Management
        │    └─ Device Virtualization
        │
        └─── Test VMs
             ├─ Node 1 (TAP0, 10.100.0.2)
             ├─ Node 2 (TAP1, 10.100.0.3)
             └─ Node 3 (TAP2, 10.100.0.4)
```

## Performance Characteristics

Based on our configuration:
- **VM Boot Time**: 100-200ms (direct kernel boot)
- **VMM Overhead**: ~110MB per VM (4vCPU, 1GB RAM)
- **Density**: 50-200 VMs per host possible
- **Network**: Near-native throughput with TAP devices

## Troubleshooting

### KVM Not Available
```bash
# Check KVM support
ls /dev/kvm

# On NixOS, enable in configuration.nix:
virtualisation.libvirtd.enable = true;
```

### Network Bridge Issues
```bash
# Check bridges
ip link show type bridge

# Manually create if needed
sudo ip link add aspen-br0 type bridge
sudo ip addr add 10.100.0.1/24 dev aspen-br0
sudo ip link set aspen-br0 up
```

### Permission Denied on TAP Devices
```bash
# Add user to appropriate groups
sudo usermod -aG kvm,libvirtd $USER

# Re-login for group changes to take effect
```

## Custom Build Options

To modify the Cloud Hypervisor build:

1. Edit `flake.nix`:
```nix
cloud-hypervisor-custom = craneLib.buildPackage {
  # ...
  cargoExtraArgs = "--features kvm,io_uring,your_feature";
};
```

2. Rebuild:
```bash
nix build .#cloud-hypervisor-custom
```

3. Use custom build:
```bash
./result/bin/cloud-hypervisor --version
```

## References

- [Cloud Hypervisor Documentation](https://github.com/cloud-hypervisor/cloud-hypervisor/tree/main/docs)
- [REST API Reference](https://github.com/cloud-hypervisor/cloud-hypervisor/blob/main/docs/api.md)
- [Aspen VM Testing Guide](./CLOUD_HYPERVISOR_QUICK_REFERENCE.md)