# Cloud Hypervisor: Comprehensive Technical Analysis

## Executive Summary

Cloud Hypervisor is a lightweight, open-source virtual machine monitor (VMM) written in Rust, designed for cloud-native workloads and high-density VM deployment. It provides excellent isolation, performance, and is ideal for spawning lightweight VMs for distributed systems testing like Aspen. The codebase uses KVM (and optionally Microsoft Hyper-V) as the underlying hypervisor and implements a complete virtualization stack from CPU/memory management to virtio device emulation.

---

## 1. Overall Architecture and Structure

### 1.1 Core Components

Cloud Hypervisor is organized as a Rust workspace with focused, modular crates:

**Main Crates:**

- **`cloud-hypervisor`** - Binary entry point with CLI parsing and VMM bootstrap
- **`vmm`** - Virtual Machine Manager core logic (3633 lines)
  - VM lifecycle management (create, boot, pause, resume, delete)
  - Device manager for virtio and vhost-user devices
  - Memory and CPU management
  - API handling (REST/HTTP and D-Bus)
  - VM configuration parsing
  - Migration and snapshot support

- **`hypervisor`** - Hypervisor abstraction layer
  - Traits for KVM and Microsoft Hyper-V (MSHV)
  - CPU/vCPU management
  - Device I/O event handling
  - Nested virtualization support

- **`devices`** - Device emulation
  - Legacy devices: serial port, RTC/CMOS, I/O APIC, ACPI
  - Virtio devices: blk, console, iommu, net, pmem, rng, vsock
  - Vhost-user backends for offloaded I/O

- **`memory_manager`** - Memory allocation and management
  - Guest physical memory mapping
  - NUMA support
  - Hugepage configuration
  - Memory ballooning
  - Memory hotplug

- **`cpu`** - vCPU management
  - CPU topology configuration
  - Affinity pinning
  - Feature flags (AMX, nested virtualization)
  - CPU hotplug

- **`api`** - API interfaces
  - REST API (HTTP over Unix socket)
  - D-Bus API (optional feature)
  - Internal MPSC-based API for inter-thread communication

**Supporting Crates:**

- `api_client` - REST API client library
- `arch` - Architecture-specific code (x86_64, ARM64, RISC-V)
- `net_util` - Networking utilities (TAP device handling)
- `net_gen` - Network code generation
- `pci` - PCI device management
- `block` - Block device backend
- `vhost_user_net` - Vhost-user network backend
- `vhost_user_block` - Vhost-user block backend
- `virtio-devices` - Virtio device implementations
- `vm-device` - VM device abstractions
- `vm-memory` - Guest memory management
- `vm-virtio` - Virtio abstractions
- `vm-migration` - Live migration support
- `vm-allocator` - Resource allocation
- `test_infra` - Integration test infrastructure

### 1.2 Architectural Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                   cloud-hypervisor CLI/Binary               │
├─────────────────────────────────────────────────────────────┤
│                      VMM (Virtual Machine Manager)          │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  API Layer (REST/HTTP, D-Bus, CLI)                    │ │
│  └────────────────────────────────────────────────────────┘ │
│  ┌─────────────┬──────────────┬──────────────────────────┐  │
│  │ VM Lifecycle│ Device       │ Memory/CPU Management    │  │
│  │ Manager     │ Manager      │                          │  │
│  └─────────────┴──────────────┴──────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│               Hypervisor Abstraction Layer                  │
│   ┌──────────────────┬────────────────────────────────┐    │
│   │  KVM Backend     │  MSHV Backend (optional)       │    │
│   │  (Linux native)  │  (Windows Hyper-V)            │    │
│   └──────────────────┴────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│                   Devices                                   │
│  ┌─────────┬───────┬──────────┬────────┬──────────────┐   │
│  │Virtio   │Legacy │Vhost-    │VFIO    │Specialized  │   │
│  │Devices  │       │User      │        │(TPM, IOMMU) │   │
│  └─────────┴───────┴──────────┴────────┴──────────────┘   │
├─────────────────────────────────────────────────────────────┤
│              Network & Storage                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ TAP Device│ MACVTAP │ Virtio-fs │ Block Devices    │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. API Surface

### 2.1 REST API

**Location:** Unix socket (default: `/tmp/cloud-hypervisor.sock`)

**Transport:** HTTP/1.1 over Unix socket, OpenAPI 3.0 compliant

**Key Endpoints:**

| Category | Endpoint | Method | Purpose |
| -------- | -------- | ------ | ------- |
| **VMM** | `/vmm.ping` | GET | Health check |
| | `/vmm.shutdown` | PUT | Graceful VMM shutdown |
| **VM Lifecycle** | `/vm.create` | PUT | Create VM with config |
| | `/vm.boot` | PUT | Boot the VM |
| | `/vm.shutdown` | PUT | Graceful VM shutdown |
| | `/vm.reboot` | PUT | Reboot VM |
| | `/vm.delete` | DELETE | Delete VM |
| | `/vm.pause` | PUT | Pause VM (live migration prep) |
| | `/vm.resume` | PUT | Resume paused VM |
| | `/vm.power-button` | PUT | Trigger ACPI power button |
| **VM Info** | `/vm.info` | GET | Get VM configuration/status |
| | `/vm.counters` | GET | Get performance counters |
| | `/vm.nmi` | POST | Inject NMI interrupt |
| **Device Management** | `/vm.add-disk` | PUT | Add disk device (hotplug) |
| | `/vm.add-net` | PUT | Add network device (hotplug) |
| | `/vm.add-fs` | PUT | Add filesystem (virtio-fs) |
| | `/vm.add-vsock` | PUT | Add VSOCK device |
| | `/vm.add-pmem` | PUT | Add persistent memory |
| | `/vm.add-vdpa` | PUT | Add vDPA device |
| | `/vm.add-device` | PUT | Add VFIO PCI device |
| | `/vm.remove-device` | DELETE | Remove device |
| **Resource Scaling** | `/vm.resize` | PUT | Hotplug/remove CPU/memory |
| | `/vm.resize-zone` | PUT | Resize memory zone |
| **Migration** | `/vm.snapshot` | PUT | Take VM snapshot |
| | `/vm.restore` | PUT | Restore from snapshot |
| | `/vm.send-migration` | PUT | Live migrate to target |
| | `/vm.receive-migration` | PUT | Prepare to receive migration |

**Example: Create VM**

```json
PUT /api/v1/vm.create HTTP/1.1
Content-Length: 456

{
  "cpus": {"boot_vcpus": 4, "max_vcpus": 8},
  "memory": {"size": 1073741824},
  "disks": [{"path": "/path/to/disk.raw"}],
  "net": [{"ip": "192.168.1.10", "mask": "255.255.255.0", "mac": "12:34:56:78:90:01"}],
  "kernel": "/path/to/vmlinux",
  "cmdline": "console=hvc0 root=/dev/vda1 rw",
  "rng": {"src": "/dev/urandom"}
}
```

### 2.2 D-Bus API

**Optional feature:** Must compile with `--features dbus_api`

**Service Name:** User-configurable (e.g., `org.cloudhypervisor.DBusApi1`)

**Interface:** Mirrors REST API with JSON payloads, adds event signal for async notifications

### 2.3 CLI Interface

**Bootstrap only** - VM configuration passed via command-line flags, not controlled post-boot.

**Key Flags:**

```bash
--api-socket path=/tmp/ch.sock           # REST API socket
--kernel <path>                           # Kernel image
--disk path=<path>                        # Disk image (multiple allowed)
--net ip=<ip>,mask=<mask>,mac=<mac>     # Network config
--cpus boot=4,max=8,topology=...        # CPU configuration
--memory size=1G,shared=on,hugepages=on # Memory config
--fs socket=/tmp/virtiofs,tag=myfs      # Virtio-fs
--vsock cid=3,socket=/tmp/ch.vsock      # VSOCK device
--rng src=/dev/urandom                   # Random number generator
--console off                             # Disable console
--serial tty                              # Serial port
--watchdog                                # Enable watchdog
--log-file <path>                        # Log output location
-v, -vv, -vvv                           # Verbosity levels
```

### 2.4 API Client Library

**Crate:** `api_client`

Simple HTTP client for REST API:

```rust
pub fn simple_api_full_command_with_fds<T: Read + Write + ScmSocket>(
    socket: &mut T,
    method: &str,
    full_command: &str,
    request_body: Option<&str>,
    request_fds: &[RawFd],
) -> Result<(), Error>
```

---

## 3. VM Creation and Management Programmatically

### 3.1 VM Lifecycle States

```
Not Created
    ↓
    └──[/vm.create]──→ Created
                          ↓
                          └──[/vm.boot]──→ Booted
                                            ├──[/vm.pause]──→ Paused
                                            │                  ↓
                                            │          [/vm.resume]
                                            │                  ↓
                                            └──[/vm.shutdown|reboot]

                          └──[/vm.delete]──→ (VM removed)
```

### 3.2 VmConfig Structure

Main configuration structure from `vmm/src/vm_config.rs`:

```rust
pub struct VmConfig {
    pub cpus: CpusConfig,
    pub memory: MemoryConfig,
    pub payload: Option<PayloadConfig>,  // Kernel + cmdline
    pub disks: Option<Vec<DiskConfig>>,
    pub net: Option<Vec<NetConfig>>,
    pub rng: RngConfig,
    pub balloon: Option<BalloonConfig>,
    pub fs: Option<Vec<FsConfig>>,
    pub pmem: Option<Vec<PmemConfig>>,
    pub console: ConsoleConfig,
    pub devices: Option<Vec<DeviceConfig>>,
    pub user_devices: Option<Vec<UserDeviceConfig>>,
    pub vsock: Option<VsockConfig>,
    pub ivshmem: Option<IvshmemConfig>,
    pub numa: Option<Vec<NumaConfig>>,
    pub iommu: bool,
    pub serial: SerialConfig,
    pub // ... many more configuration options
}
```

### 3.3 VM Creation Flow (REST API)

1. **Initialize VMM:** Start `cloud-hypervisor` with `--api-socket` (no VM config)
2. **Create VM:** PUT `/vm.create` with JSON `VmConfig`
3. **Boot VM:** PUT `/vm.boot` (triggers guest kernel execution)
4. **Query Status:** GET `/vm.info` (returns config + current state)
5. **Manage Resources:**
   - Add devices: PUT `/vm.add-disk`, `/vm.add-net`, etc.
   - Hotplug CPU/Memory: PUT `/vm.resize`
6. **Shutdown/Cleanup:** PUT `/vm.shutdown` → DELETE `/vm.delete`

### 3.4 Key VM Management APIs

**VM Manager Thread:**

- Central MPSC channel-based state machine
- Processes API requests asynchronously
- Manages device state, CPU execution, memory

**VM Struct (vmm/src/vm.rs):**

- ~3600 lines
- Manages VM lifecycle, device initialization, boot sequence
- CPU management, memory layout
- Device I/O and event handling

---

## 4. Network Isolation Capabilities

### 4.1 Network Modes

**Virtio-Net (Primary)**

- Default paravirtualized network device
- Uses TAP interface (automatically created by Cloud Hypervisor)
- Requires `CAP_NET_ADMIN` capability
- Isolated per VM via separate TAP device

**Configuration Example:**

```json
{
  "net": [{
    "ip": "192.168.100.10",
    "mask": "255.255.255.0",
    "mac": "12:34:56:78:90:01",
    "tap": "ch-tap0",
    "num_queues": 2,
    "queue_size": 256,
    "iommu": false
  }]
}
```

### 4.2 Advanced Network Options

**MACVTAP (Bridged Networking)**

- Connects guest directly to host network
- Useful for cloud-like environments
- No hairpin mode limitation with modern kernels

**Vhost-User-Net**

- Offloads network I/O to external daemon
- For advanced isolation and performance requirements

**Network Namespace Isolation**

- Each VM can use separate TAP interface in host namespace
- Full L2/L3 isolation via host network stack

### 4.3 Network Configuration Details

```rust
pub struct NetConfig {
    pub tap: Option<String>,              // TAP device name
    pub ip: Option<IpAddr>,              // Guest IP
    pub mask: Option<IpAddr>,            // Netmask
    pub mac: MacAddr,                    // MAC address
    pub host_mac: Option<MacAddr>,       // Host TAP MAC
    pub mtu: Option<u16>,                // MTU size
    pub iommu: bool,                     // IOMMU protection
    pub num_queues: usize,               // Virtio queue count
    pub queue_size: u16,                 // Queue size
    pub vhost_user: bool,                // Use vhost-user backend
    pub vhost_socket: Option<String>,    // Vhost socket path
    pub fds: Option<Vec<i32>>,           // File descriptors for FD-based net
}
```

### 4.4 Security Considerations

- **TAP isolation:** Each VM gets dedicated TAP interface, isolated from host
- **VFIO:** Hardware device passthrough with IOMMU isolation (virtio-iommu)
- **Seccomp:** Syscall filtering to restrict VMM attack surface
- **Landlock:** Filesystem isolation via LSM
- **Nested virtualization:** Controlled access to hardware features

---

## 5. Boot Process and Guest Image Requirements

### 5.1 Boot Methods

**Direct Kernel Boot (Recommended for Testing)**

- Boot Linux kernel directly without firmware
- Minimal boot time (~100-200ms)
- Fast iteration for testing

**Firmware Boot (UEFI/BIOS)**

- OVMF firmware (x86_64)
- UEFI firmware (ARM64)
- Required for Windows guests

**Boot Flow:**

```
1. VMM creates VM, initializes KVM structures
2. Loads kernel image into guest RAM at specified address
3. Constructs Linux bootloader protocol structures
4. Sets vCPU entry point (rip for x86_64, pc for ARM64)
5. vCPUs begin execution from entry point
6. Kernel decompresses, initializes, mounts root filesystem
7. Init process starts
```

### 5.2 Guest Image Requirements

**Linux Guests:**

- Cloud images (Ubuntu, Debian, CentOS, etc.)
- Format: RAW, QCOW2, VHD, VHDX
- Recommended: Ubuntu Jammy or later
- Partition layout: `/dev/vda` (virtio-blk)

**Windows Guests:**

- Windows Server 2022+ (tested)
- Windows 11 IoT Enterprise (ARM64 tested)
- Requires OVMF UEFI firmware
- Requires KVM Hyper-V synthetic device support

**Kernel Requirements:**

- Linux 5.10+ for virtio-fs support
- Linux 5.5+ for VSOCK nested VM support
- Specific features: `CONFIG_VIRTIO_*`, `CONFIG_VHOST_*`, `CONFIG_VFIO`

### 5.3 Boot Configuration

**Kernel and Command Line:**

```json
{
  "payload": {
    "kernel": "/path/to/vmlinux-5.15.0",
    "initramfs": "/path/to/initrd.img",
    "cmdline": "console=hvc0 root=/dev/vda1 rw systemd.journald.forward_to_console=1"
  }
}
```

**Common Cmdline Parameters:**

- `console=hvc0` - Use virtio-console
- `console=ttyS0` - Use serial port (x86_64)
- `console=ttyAMA0` - Use serial port (ARM64)
- `root=/dev/vda1` - Root filesystem location
- `ro` / `rw` - Read-only or read-write
- `quiet` - Suppress boot messages
- `systemd.journald.forward_to_console=1` - Forward journal to console

### 5.4 Custom Image Creation

**Build Script:** `scripts/build-custom-image.sh` (in Cloud Hypervisor repo)

**Process:**

1. Start from official cloud image
2. Mount and modify filesystem
3. Install additional packages/tools
4. Repackage as RAW/QCOW2

---

## 6. Memory and CPU Configuration Options

### 6.1 CPU Configuration

**CpusConfig Structure:**

```rust
pub struct CpusConfig {
    pub boot_vcpus: u8,              // vCPUs at boot (1-255)
    pub max_vcpus: u8,               // Max vCPUs (>= boot_vcpus)
    pub topology: Option<CpuTopology>, // SMT/cores/dies/sockets
    pub kvm_hyperv: bool,            // KVM Hyper-V extensions
    pub max_phys_bits: u8,           // Physical address space
    pub affinity: Option<Vec<CpuAffinity>>, // CPU pinning
    pub features: CpuFeatures,       // Feature flags (AMX)
    pub nested: bool,                // Nested virtualization
}

pub struct CpuTopology {
    pub threads_per_core: u8,
    pub cores_per_die: u8,
    pub dies_per_package: u8,
    pub packages: u8,
}

pub struct CpuAffinity {
    pub vcpu: u8,
    pub host_cpus: Vec<u8>,
}
```

**Examples:**

```bash
# 4 vCPUs at boot, max 8
--cpus boot=4,max=8

# Topology: 2 threads/core, 2 cores/die, 1 die, 1 package = 4 vCPUs total
--cpus boot=4,topology=2:2:1:1

# Pin vCPU 0 to host CPUs 0-3, vCPU 1 to host CPUs 4-7
--cpus boot=2,affinity=[0@[0-3],1@[4-7]]

# Enable nested virtualization
--cpus boot=4,nested=on

# Enable AMX CPU feature
--cpus boot=4,features=amx

# Enable KVM Hyper-V (for Windows)
--cpus boot=4,kvm_hyperv=on
```

**CPU Hotplug:**

- Add/remove vCPUs at runtime via `/vm.resize`
- Max vCPU must be set at VM creation
- ACPI or virtio-mem-based addition (guest OS dependent)

### 6.2 Memory Configuration

**MemoryConfig Structure:**

```rust
pub struct MemoryConfig {
    pub size: u64,                   // Base memory size (bytes)
    pub mergeable: bool,             // KSM eligible
    pub hotplug_method: HotplugMethod, // acpi or virtio-mem
    pub hotplug_size: Option<u64>,   // Max hotpluggable memory
    pub hotplugged_size: Option<u64>, // Pre-hotplugged amount
    pub shared: bool,                // MAP_SHARED for vhost devices
    pub hugepages: bool,             // Use hugepages (2M/1G)
    pub hugepage_size: Option<u64>,  // Explicit hugepage size
    pub prefault: bool,              // MAP_POPULATE (pre-fault pages)
    pub thp: bool,                   // Transparent huge pages
    pub zones: Option<Vec<MemoryZoneConfig>>, // NUMA zones
}
```

**Examples:**

```bash
# Basic: 512MB RAM
--memory size=512M

# 1GB with transparent huge pages (default, recommended)
--memory size=1G,thp=on

# 2GB with explicit 2M hugepages
--memory size=2G,hugepages=on,hugepage_size=2M

# 4GB shared with vhost devices + mergeable
--memory size=4G,shared=on,mergeable=on

# 1GB base + 3GB hotpluggable
--memory size=1G,hotplug_size=3G,hotplug_method=virtio-mem

# NUMA-aware: 2 zones of 2GB each on different nodes
--memory size=4G,zones=[size=2G,numa_node=0,size=2G,numa_node=1]
```

**Memory Hotplug:**

- ACPI method: Add/remove in 256MB chunks (guest-controlled)
- Virtio-mem: More flexible, guest can request arbitrary amounts
- Add/remove via `/vm.resize` REST API

### 6.3 NUMA Configuration

**NumaConfig:**

```rust
pub struct NumaConfig {
    pub guest_numa_id: u16,          // Guest NUMA node ID
    pub host_numa_nodes: Vec<u16>,   // Host NUMA nodes
    pub cpus: Option<Vec<u8>>,       // vCPU assignments
    pub distances: Option<Vec<NumaDistance>>, // Latencies
    pub memory_zones: Option<Vec<u16>>, // Memory zone IDs
}
```

---

## 7. Storage and Filesystem Options

### 7.1 Virtio-Blk (Block Storage)

**Primary storage device**

**Configuration:**

```json
{
  "disks": [{
    "path": "/path/to/disk.raw",
    "readonly": false,
    "direct": false,
    "num_queues": 1,
    "queue_size": 256,
    "iommu": false,
    "rate_limiter_config": {...}
  }]
}
```

**Supported Formats:**

- RAW: Raw image file (fastest, no compression)
- QCOW2: Copy-on-write (backing files, compression)
- VHD: Virtual Hard Disk (Azure, Hyper-V)
- VHDX: VHD Extended (newer, larger images)

**Features:**

- Hotplug: Add/remove disks at runtime
- Rate limiting: I/O bandwidth/ops throttling
- IOMMU: Hardware protection via virtio-iommu
- Direct I/O: Bypass page cache

### 7.2 Virtio-FS (Filesystem Sharing)

**Shared filesystem between host and guest**

**Architecture:**

```
Host Filesystem
      ↓
virtiofsd daemon (vhost-user backend)
      ↓
VM (VSOCK or socket)
      ↓
Guest (virtiofs mount)
```

**Prerequisites:**

- Linux kernel 5.10+
- virtiofsd daemon on host: `git clone https://gitlab.com/virtio-fs/virtiofsd`
- Shared memory: `--memory shared=on`

**Configuration:**

```json
{
  "fs": [{
    "socket": "/tmp/virtiofs.sock",
    "tag": "myfs",
    "num_queues": 1,
    "queue_size": 512,
    "dax": false
  }]
}
```

**Guest Mount:**

```bash
mkdir /mnt/host
mount -t virtiofs myfs /mnt/host
```

**Performance Options:**

- `cache=never` (default) - No host page cache, lower memory footprint
- `cache=always` - Use host page cache, better performance
- `thread_pool_size` - Number of I/O worker threads

### 7.3 Virtio-PMEM (Persistent Memory)

**Virtual persistent memory device**

**Use Cases:**

- Database workloads expecting PMEM
- Bypass guest page cache
- Direct memory mapping

**Configuration:**

```json
{
  "pmem": [{
    "file": "/path/to/pmem.img",
    "size": 1073741824,  // 1GB
    "iommu": false
  }]
}
```

### 7.4 Vhost-User Block (Offloaded Backend)

**External block backend daemon** (e.g., SPDK)

**Configuration:**

```json
{
  "disks": [{
    "vhost_user": true,
    "socket": "/tmp/vhost-user-blk.sock",
    "readonly": false
  }]
}
```

### 7.5 Mount and Boot Options

**Root Filesystem:**

```bash
# Direct kernel boot with root=/dev/vda1
--cmdline "root=/dev/vda1 rw"

# Alternative partitions
--cmdline "root=/dev/vda2 rw"

# With cloud-init
--cmdline "root=/dev/vda1 rw cloud-init=enabled"
```

---

## 8. Rust SDK and Library Interface

### 8.1 Programmatic VM Creation

**Primary Crate:** `vmm`

**Example Usage Pattern:**

```rust
use vmm::config::VmConfig;
use vmm::Vmm;

// 1. Create configuration
let config = VmConfig {
    cpus: CpusConfig {
        boot_vcpus: 4,
        max_vcpus: 8,
        ..Default::default()
    },
    memory: MemoryConfig {
        size: 1024 * 1024 * 1024, // 1GB
        ..Default::default()
    },
    payload: Some(PayloadConfig {
        kernel: "/path/to/vmlinux".into(),
        cmdline: Some("root=/dev/vda1 rw".into()),
        ..Default::default()
    }),
    disks: Some(vec![DiskConfig {
        path: "/path/to/disk.raw".into(),
        ..Default::default()
    }]),
    ..Default::default()
};

// 2. Initialize hypervisor
let hypervisor = hypervisor::new()?;

// 3. Create VM
let vm = Vm::new(&hypervisor, &config)?;

// 4. Boot VM
vm.boot()?;

// 5. Manage at runtime
// ... use REST API for hotplug, resize, etc.
```

### 8.2 API Client Library

**Crate:** `api_client`

```rust
use api_client::{simple_api_full_command, StatusCode};
use std::os::unix::net::UnixStream;

fn create_vm(socket_path: &str, config_json: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut socket = UnixStream::connect(socket_path)?;
    simple_api_full_command(
        &mut socket,
        "PUT",
        "vm.create",
        Some(config_json),
    )?;
    Ok(())
}
```

### 8.3 Test Infrastructure

**Crate:** `test_infra`

Provides high-level abstraction for integration testing:

```rust
use test_infra::{Guest, GuestCommand, UbuntuDiskConfig};

let disk_config = UbuntuDiskConfig::new("jammy-server-cloudimg-amd64.raw".into());
let guest = Guest::new(disk_config, temp_dir);

let mut child = GuestCommand::new(&guest)
    .args(&["--cpus", "boot=4"])
    .args(&["--memory", "size=1G"])
    .spawn()?;

// Wait for VM to boot
guest.wait_vm_boot(None)?;

// SSH into guest
guest.ssh_command("uname -a")?;

// Check resources
let cpu_count = guest.get_cpu_count()?;
let memory = guest.get_total_memory()?;
```

### 8.4 Important Traits and Traits

**Core Traits:**

- `Migratable` - VM snapshot/restore capability
- `Snapshottable` - State serialization
- `Pausable` - VM pause/resume
- `VmOps` - Hypervisor operations (create vCPU, attach device, etc.)
- `Vm` - Main VM abstraction from hypervisor layer

---

## 9. Build Requirements and Dependencies

### 9.1 System Requirements

**Build:**

- Rust 1.75+ (Rust 2024 edition)
- Linux kernel headers (for KVM ioctl bindings)
- Build essentials: gcc, make, pkg-config

**Runtime:**

- Linux kernel with KVM enabled (or MSHV on Windows)
- CAP_NET_ADMIN capability (for TAP device creation)
- CAP_SYS_ADMIN (optional, for certain features)

### 9.2 Key Dependencies

**Hypervisor:**

- `kvm-bindings 0.12.1` - KVM ioctl bindings
- `kvm-ioctls 0.22.1` - KVM operations
- `mshv-bindings 0.6.5` - Microsoft Hyper-V bindings (optional)
- `hypervisor` crate (internal) - Abstraction layer

**Virtualization Stack:**

- `linux-loader 0.13.1` - Kernel/initramfs loading
- `vm-memory 0.16.1` - Guest memory management
- `vm-fdt 0.3.0` - Device tree generation (ARM64)
- `virtio-bindings 0.2.6` - Virtio device specs
- `virtio-queue 0.16.0` - Virtio queue implementation

**I/O and Networking:**

- `vhost 0.14.0` - Vhost protocol
- `vhost-user-backend 0.20.0` - Vhost-user backend support
- `vfio-bindings 0.6.0` - VFIO bindings
- `net_util` (internal) - Network utilities

**API:**

- `micro_http` (Firecracker fork) - HTTP server
- `serde 1.0.228` - Serialization
- `serde_json 1.0.145` - JSON handling
- `zbus` (optional, for D-Bus API)

**Utilities:**

- `libc 0.2.178` - C library bindings
- `log 0.4.29` - Logging
- `seccompiler 0.5.0` - Seccomp filter compilation
- `signal-hook 0.3.18` - Signal handling
- `flume 0.12.0` - MPSC channels

### 9.3 Build Configuration

**Profiles:**

```toml
[profile.release]
codegen-units = 1   # Single-threaded codegen for better optimization
lto = true          # Link-time optimization
opt-level = "s"     # Optimize for size
strip = true        # Remove debug symbols

[profile.profiling]  # For profiling builds
debug = true        # Keep debug symbols
inherits = "release"
strip = false
```

**Feature Flags:**

```toml
# Optional features
kvm          # KVM support (default)
mshv         # Microsoft Hyper-V support
dbus_api     # D-Bus API support
fw_cfg       # Firmware config device
ivshmem      # Inter-VM shared memory
guest_debug  # GDB support
sev_snp      # AMD SEV-SNP support
tdx          # Intel TDX support
igvm         # IGVM (Isolated Guest VM) support
```

### 9.4 Build Commands

```bash
# Standard release build
cargo build --release

# Static musl build (portable)
cargo build --release --target x86_64-unknown-linux-musl

# With optional features
cargo build --release --features dbus_api,fw_cfg

# Development build with logging
RUST_LOG=debug cargo build

# Post-build capability setup
sudo setcap cap_net_admin+ep ./target/release/cloud-hypervisor
```

---

## 10. Performance Characteristics and Overhead

### 10.1 VM Startup Time

**Typical Boot Times:**

- Minimal direct kernel boot: 100-200ms
- Full Linux boot (systemd): 5-15 seconds
- Windows UEFI boot: 30-60 seconds

**Factors:**

- Kernel size and complexity
- Initramfs size
- Device initialization overhead
- Guest I/O during boot

### 10.2 Memory Footprint

**Per-VM Overhead (after boot):**

- Base: ~50-100 MB (VMM process + kernel structures)
- Per vCPU: ~2-4 MB (stack, TLS, structures)
- Per device: ~1-5 MB (device state, queues)

**Example (4 vCPU, 1GB RAM VM):**

```
Host Memory Usage:
  VMM + kernel structures: 80 MB
  4 vCPUs: 16 MB
  Devices (3): 10 MB
  Total VMM: ~110 MB
  Guest RAM: 1024 MB
  ---
  Total: ~1134 MB
```

**Optimization:**

- Mergeable pages (KSM): 10-20% reduction with idle guests
- Transparent huge pages: Faster page table traversals
- Hugepages: Better TLB efficiency

### 10.3 CPU Overhead

**Direct Execution:**

- VM workloads run directly on hardware
- Minimal context switch overhead
- ~2-5% overhead for I/O-heavy workloads

**Factors:**

- Number of vCPUs (scaling effects)
- I/O rate (EPT/NPT page walks)
- Exit frequency (device emulation, interrupts)

### 10.4 Network Performance

**Virtio-Net Characteristics:**

- Throughput: Near-native (>90% wire speed)
- Latency: +100-500μs due to device emulation
- Scaling: Multi-queue (4-8 queues for optimal throughput)

**Optimization Tips:**

- Use multiple queues: `--net ... num_queues=4`
- Enable batching in guest kernel
- Use vhost-user for offloaded backends

### 10.5 Storage I/O Performance

**Virtio-Blk:**

- Throughput: 90-98% native (HDD), 85-95% (SSD)
- Latency: +10-50μs per I/O
- Scalability: Multiple queues and queue depth

**File Format Impact:**

- RAW: Fastest, direct access
- QCOW2: 5-10% overhead (COW, compression handling)
- VHD/VHDX: 2-5% overhead

**Optimization:**

- Use RAW images for performance-critical workloads
- Enable direct I/O: `--disk path=... direct=on`
- Increase queue depth for parallel I/O

### 10.6 Density and Scalability

**High-Density Deployment:**

- Per-host VM limits: 50-200 VMs (depends on kernel limits)
- Memory density: 100-500GB per host with overcommit
- CPU density: Can run 4-10x more vCPUs than physical cores

**Achieved With:**

- Memory sharing (KSM, THP)
- CPU overcommit (careful scheduling)
- Small base images (~300-500 MB)
- Minimal guest overhead

---

## 11. Key Features Summary

### 11.1 Supported Platforms

- **x86_64** (KVM or MSHV) - Full support
- **ARM64/AArch64** (KVM) - Full support, UEFI firmware
- **RISC-V 64** (KVM) - Experimental

### 11.2 Device Support Matrix

| Device | Built-in | Default | Runtime Config |
| ------ | -------- | ------- | -------------- |
| Serial Port | Yes | No | ✓ |
| RTC/CMOS | Yes | Yes | ✗ |
| I/O APIC | Yes | Conditional | ✓ |
| ACPI | Yes | Yes | ✗ |
| Virtio-Blk | Yes | Yes (w/ disk) | ✓ (hotplug) |
| Virtio-Console | Yes | Yes | ✓ |
| Virtio-Net | Yes | Yes (w/ net) | ✓ (hotplug) |
| Virtio-IOMMU | Yes | Conditional | ✓ |
| Virtio-PMEM | Yes | Yes (w/ pmem) | ✓ (hotplug) |
| Virtio-RNG | Yes | Yes | ✓ |
| Virtio-VSOCK | Yes | Yes (w/ vsock) | ✓ (hotplug) |
| Vhost-User-Blk | Yes | Conditional | ✓ |
| Vhost-User-FS | Yes | Conditional | ✓ |
| Vhost-User-Net | Yes | Conditional | ✓ |
| VFIO (PCI) | Build feature | ✗ | ✓ (hotplug) |

### 11.3 Advanced Features

- **Live Migration** - Pause, snapshot, transport, restore
- **Memory Hotplug** - ACPI and virtio-mem methods
- **CPU Hotplug** - Dynamic vCPU addition
- **Device Hotplug** - Add/remove storage, network, custom devices
- **Nested Virtualization** - L2 guests (KVM in KVM)
- **NUMA** - Multi-node guest topology with affinity
- **Snapshot & Restore** - Full VM state capture/replay
- **Paravirtualization** - virtio, vhost-user offloading
- **Security Features** - Seccomp, Landlock, SEV-SNP, TDX support
- **Live Boot Updates** - Custom kernel at boot (no firmware)

---

## 12. Use Case for Aspen (Distributed Systems Testing)

### 12.1 Why Cloud Hypervisor for Aspen?

**Strengths:**

1. **Lightweight** - Minimal per-VM overhead, high density testing
2. **Fast boot** - Direct kernel boot: 100-200ms for test setup
3. **Programmable** - REST API + Rust SDK for test orchestration
4. **Isolation** - TAP network, VFIO, seccomp for security
5. **Performance** - Near-native CPU/network/storage for realistic tests
6. **Reproducible** - Deterministic boot, snapshot/restore capability

### 12.2 Ideal Deployment Pattern

```
Aspen Test Cluster
    ↓
Control Plane (Test Orchestrator)
    ├─[REST API]──→ Cloud Hypervisor #1
    │                   ├─ Node 1 (VM)
    │                   ├─ Node 2 (VM)
    │                   ├─ Node 3 (VM)
    │                   └─ Node 4 (VM)
    │
    ├─[REST API]──→ Cloud Hypervisor #2
    │                   ├─ Node 5 (VM)
    │                   ├─ Node 6 (VM)
    │                   └─ Node 7 (VM)
    │
    └─[REST API]──→ Cloud Hypervisor #N
                        └─ ... more nodes
```

**Test Flow:**

1. Spin up N VMs (distributed Raft cluster)
2. Perform operations (writes, reads, rebalancing)
3. Inject faults (network partitions via TAP manipulation)
4. Verify convergence, consistency
5. Collect metrics/logs from guest nodes
6. Clean up (snapshot state for debugging if needed)

### 12.3 Configuration Example for Aspen

```bash
# For 5-node Raft cluster, each node:
# - 2 vCPU (sufficient for distributed system testing)
# - 512MB RAM (minimal for Linux + test workload)
# - 1GB disk (root filesystem + state)
# - VSOCK for control communication
# - Isolated network (TAP per node)

# Node 1
cloud-hypervisor \
  --api-socket /tmp/ch1.sock \
  --cpus boot=2 \
  --memory size=512M \
  --disk path=/tmp/node1.raw \
  --kernel /path/to/vmlinux \
  --cmdline "root=/dev/vda1 rw console=hvc0" \
  --net ip=192.168.1.11,mac=12:34:56:78:90:11 \
  --vsock cid=3,socket=/tmp/node1.vsock \
  --serial off \
  --console off

# Control plane creates/manages via REST API
curl --unix-socket /tmp/ch1.sock \
  -X PUT http://localhost/api/v1/vm.create \
  -H "Content-Type: application/json" \
  -d @node1-config.json

curl --unix-socket /tmp/ch1.sock \
  -X PUT http://localhost/api/v1/vm.boot

# Later: inject network fault, pause, snapshot, etc.
```

---

## Conclusion

Cloud Hypervisor provides an excellent foundation for Aspen's distributed systems testing infrastructure. Its lightweight architecture, programmable APIs, and strong isolation properties make it ideal for spawning hundreds of isolated test nodes efficiently. The combination of direct kernel boot, REST API control, and Rust SDK integration aligns perfectly with Aspen's goals for reproducible, high-performance distributed system simulation.

Key advantages for Aspen:

- **Rapid iteration:** 100-200ms boot times enable fast test cycles
- **Scale:** 50-200 VMs per host for realistic cluster scenarios
- **Control:** REST API for programmatic test orchestration
- **Isolation:** Network, filesystem, and compute isolation per VM
- **Debugging:** Snapshots, logs, and deterministic replay
