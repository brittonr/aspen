//! CI/CD constants for Aspen distributed system.
//!
//! This module contains constants for CI job execution, VM resource limits,
//! log streaming, and Cloud Hypervisor microVM configuration.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.

// ============================================================================
// CI Job Resource Limits
// ============================================================================
// These constants control resource isolation for CI job execution to prevent
// CI processes from exhausting system resources and starving Raft consensus.

/// Maximum memory per CI job (4 GB).
///
/// Tiger Style: Fixed limit prevents single CI job from exhausting system memory.
/// Tests and builds typically need 1-4GB; 4GB is generous while still protective.
///
/// Used in:
/// - `aspen-ci/workers/resource_limiter.rs`: cgroup memory.max setting
pub const MAX_CI_JOB_MEMORY_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Soft memory limit per CI job for throttling (3 GB).
///
/// Tiger Style: Soft limit triggers memory reclaim before hard limit OOM.
/// Set at 75% of MAX_CI_JOB_MEMORY_BYTES to give early warning.
///
/// Used in:
/// - `aspen-ci/workers/resource_limiter.rs`: cgroup memory.high setting
pub const MAX_CI_JOB_MEMORY_HIGH_BYTES: u64 = 3 * 1024 * 1024 * 1024;

/// CPU weight for CI jobs (relative priority).
///
/// Tiger Style: Lower weight gives Raft consensus priority over CI jobs.
/// Default cgroup weight is 100; CI jobs get lower priority.
///
/// Used in:
/// - `aspen-ci/workers/resource_limiter.rs`: cgroup cpu.weight setting
pub const CI_JOB_CPU_WEIGHT: u32 = 50;

/// Maximum PIDs per CI job.
///
/// Tiger Style: Fixed limit prevents fork bombs and runaway test parallelism.
/// 4096 allows significant parallelism while preventing resource exhaustion.
///
/// Used in:
/// - `aspen-ci/workers/resource_limiter.rs`: cgroup pids.max setting
pub const MAX_CI_JOB_PIDS: u32 = 4096;

/// Maximum I/O bandwidth per CI job in bytes/second (100 MB/s).
///
/// Tiger Style: Fixed limit prevents I/O-based DoS from CI jobs.
/// 100 MB/s is sufficient for most builds while protecting host disk.
///
/// Used in:
/// - `aspen-ci/workers/resource_limiter.rs`: cgroup io.max setting
pub const MAX_CI_JOB_IO_BYTES_PER_SEC: u64 = 100 * 1024 * 1024;

/// Maximum I/O operations per CI job per second (1000 IOPS).
///
/// Tiger Style: Fixed limit prevents IOPS-based DoS from CI jobs.
/// 1000 IOPS is sufficient for most builds while protecting host disk.
///
/// Used in:
/// - `aspen-ci/workers/resource_limiter.rs`: cgroup io.max setting
pub const MAX_CI_JOB_IO_OPS_PER_SEC: u64 = 1000;

// ============================================================================
// CI Log Streaming Constants
// ============================================================================
// Constants for real-time CI job log streaming to TUI clients.
// Logs are stored in KV with structured keys and streamed via WatchSession.

/// Maximum size of a single CI log chunk (8 KB).
///
/// Tiger Style: Bounded chunk size ensures predictable KV write latency.
/// 8KB balances granularity against overhead for real-time streaming.
///
/// Used in:
/// - `aspen-ci/log_writer.rs`: CiLogWriter buffer flush threshold
pub const MAX_CI_LOG_CHUNK_SIZE: u32 = 8 * 1024;

/// Maximum number of log chunks per job (10,000 chunks = ~80MB max).
///
/// Tiger Style: Bounded total log size prevents disk exhaustion.
/// 10K chunks * 8KB = 80MB max per job. Excess logs are silently dropped.
///
/// Used in:
/// - `aspen-ci/log_writer.rs`: CiLogWriter chunk count limit
pub const MAX_CI_LOG_CHUNKS_PER_JOB: u32 = 10_000;

/// CI log retention period in milliseconds (24 hours = 86,400,000 ms).
///
/// Tiger Style: Bounded retention prevents unbounded log growth.
/// Logs older than 24 hours are eligible for cleanup by garbage collection.
///
/// Used in:
/// - `aspen-ci/log_writer.rs`: Log cleanup scheduling
/// - `aspen-rpc-handlers/handlers/ci.rs`: Log expiry checks
pub const CI_LOG_RETENTION_MS: u64 = 24 * 60 * 60 * 1000;

/// CI log flush interval in milliseconds (500 ms).
///
/// Tiger Style: Bounded flush frequency balances latency vs. throughput.
/// Workers buffer logs and flush every 500ms to reduce KV write overhead.
///
/// Used in:
/// - `aspen-ci/log_writer.rs`: SpawnedLogWriter flush interval
pub const CI_LOG_FLUSH_INTERVAL_MS: u64 = 500;

/// KV prefix for CI job logs.
///
/// Key format: `{CI_LOG_KV_PREFIX}{run_id}:{job_id}:{chunk_index:010}`
/// Zero-padded index ensures lexicographic ordering matches insertion order.
///
/// Used in:
/// - `aspen-ci/log_writer.rs`: Log chunk key construction
/// - `aspen-tui/iroh_client.rs`: WatchSession subscription prefix
/// - `aspen-rpc-handlers/handlers/ci.rs`: Log scan operations
pub const CI_LOG_KV_PREFIX: &str = "_ci:logs:";

/// KV suffix marker for log stream completion.
///
/// Written when job completes to signal end-of-stream to watchers.
/// Watchers can check for this key to detect job completion.
///
/// Used in:
/// - `aspen-ci/log_writer.rs`: Completion marker write
/// - `aspen-tui/iroh_client.rs`: Stream termination detection
pub const CI_LOG_COMPLETE_MARKER: &str = "__complete__";

/// Maximum log lines retained in TUI display buffer (10,000 lines).
///
/// Tiger Style: Bounded buffer prevents unbounded memory use in TUI.
/// Older lines are dropped when limit is reached (circular buffer behavior).
///
/// Used in:
/// - `aspen-tui/types.rs`: CiLogStreamState buffer bounds
pub const MAX_TUI_LOG_LINES: usize = 10_000;

/// Maximum log chunks to fetch in a single CiGetJobLogs request (1000).
///
/// Tiger Style: Bounded result set prevents memory exhaustion.
/// Clients can paginate using start_index for more chunks.
///
/// Used in:
/// - `aspen-rpc-handlers/handlers/ci.rs`: CiGetJobLogs limit validation
pub const MAX_CI_LOG_FETCH_CHUNKS: u32 = 1000;

/// Default log chunks to fetch in CiGetJobLogs request (100).
///
/// Tiger Style: Reasonable default for initial log fetch.
/// Enough for context without overwhelming the client.
///
/// Used in:
/// - `aspen-rpc-handlers/handlers/ci.rs`: CiGetJobLogs default limit
pub const DEFAULT_CI_LOG_FETCH_CHUNKS: u32 = 100;

// ============================================================================
// Cloud Hypervisor CI VM Constants
// ============================================================================
// Constants for Cloud Hypervisor microVM-based CI job execution.
// These VMs provide full isolation for untrusted code execution.

/// Maximum number of CI VMs per node (8).
///
/// Tiger Style: Fixed limit prevents resource exhaustion on host.
/// Each VM uses significant memory and CPU; 8 is a reasonable upper bound.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/pool.rs`: VmPool capacity limits
pub const MAX_CI_VMS_PER_NODE: u32 = 8;

/// Default memory per CI VM in bytes (24 GB).
///
/// Tiger Style: Fixed default memory allocation per VM.
/// 24GB is required for large Rust builds:
/// - ~12GB for tmpfs writable store overlay (/nix/.rw-store)
/// - ~4GB for Nix evaluation (parsing 2000+ derivations)
/// - ~4GB for build processes (cargo, rustc, linker)
/// - ~4GB for system overhead
///
/// Note: Each VM also requires ~1GB virtiofsd shmem for virtiofs file sharing.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/config.rs`: Default VM memory configuration
pub const CI_VM_DEFAULT_MEMORY_BYTES: u64 = 24 * 1024 * 1024 * 1024;

/// Maximum memory per CI VM in bytes (64 GB).
///
/// Tiger Style: Upper bound prevents misconfiguration from exhausting host memory.
/// With 8 max VMs per node, worst case is 512GB if all VMs maxed out.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/config.rs`: VM memory validation
pub const CI_VM_MAX_MEMORY_BYTES: u64 = 64 * 1024 * 1024 * 1024;

/// Default vCPUs per CI VM (4).
///
/// Tiger Style: Fixed default vCPU count per VM.
/// 4 vCPUs allows parallel compilation while limiting host impact.
/// Matches dogfood VM configuration.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/config.rs`: Default VM CPU configuration
pub const CI_VM_DEFAULT_VCPUS: u32 = 4;

/// Maximum vCPUs per CI VM (16).
///
/// Tiger Style: Upper bound prevents misconfiguration.
/// Most builds are memory-bound, not CPU-bound, so 16 is generous.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/config.rs`: VM CPU validation
pub const CI_VM_MAX_VCPUS: u32 = 16;

/// CI VM boot timeout in milliseconds (60 seconds).
///
/// Tiger Style: Fixed timeout for VM boot completion.
/// Cloud Hypervisor typically boots in ~125ms, but nested virtualization
/// (CI VM inside dogfood VM) can significantly increase boot time.
/// The 180s timeout provides margin for slow virtiofs initialization
/// when running VMs inside VMs.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/vm.rs`: VM boot timeout
pub const CI_VM_BOOT_TIMEOUT_MS: u64 = 180_000;

/// Guest agent connection timeout in milliseconds (120 seconds).
///
/// Tiger Style: Fixed timeout for vsock connection to guest agent.
/// In nested virtualization (VMs inside VMs), NixOS boot with systemd
/// and virtiofs mounts can take 60+ seconds. The 120s timeout provides
/// margin for slow I/O in nested environments.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/vm.rs`: Agent ready timeout
pub const CI_VM_AGENT_TIMEOUT_MS: u64 = 120_000;

/// Default job execution timeout in CI VMs in milliseconds (30 minutes).
///
/// Tiger Style: Reasonable default for most CI jobs.
/// Nix builds can take longer; jobs can override up to max.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/worker.rs`: Default execution timeout
pub const CI_VM_DEFAULT_EXECUTION_TIMEOUT_MS: u64 = 30 * 60 * 1000;

/// Maximum job execution timeout in CI VMs in milliseconds (4 hours).
///
/// Tiger Style: Upper bound prevents indefinite VM occupation.
/// Very long builds should be split into stages.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/worker.rs`: Max execution timeout
pub const CI_VM_MAX_EXECUTION_TIMEOUT_MS: u64 = 4 * 60 * 60 * 1000;

/// Default warm VM pool size (1).
///
/// Tiger Style: Pre-warmed VMs for fast job startup.
/// 1 VM minimizes resource use while still providing fast startup.
/// Increase to 2+ for production workloads with concurrent jobs.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/pool.rs`: Pool initialization
pub const CI_VM_DEFAULT_POOL_SIZE: u32 = 1;

/// Vsock port for CI guest agent (5000).
///
/// Tiger Style: Fixed port for host-guest communication.
/// In the unprivileged range (>1024).
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/executor.rs`: Vsock connection
/// - `aspen-ci-agent/main.rs`: Vsock listener
pub const CI_VM_VSOCK_PORT: u32 = 5000;

/// Guest agent vsock CID (host is always 2).
///
/// Tiger Style: Well-known CID for host in vsock.
/// Guest connects to CID 2 to reach the host.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/executor.rs`: Vsock addressing
pub const VSOCK_HOST_CID: u32 = 2;

/// Maximum message size for guest agent protocol (16 MB).
///
/// Tiger Style: Fixed limit prevents memory exhaustion from malformed frames.
/// Job output can be large but should be bounded.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/executor.rs`: Message framing
/// - `aspen-ci-agent/protocol.rs`: Message validation
pub const CI_VM_MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

/// VM snapshot directory name.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/pool.rs`: Golden snapshot path
pub const CI_VM_SNAPSHOT_DIR: &str = "snapshots";

/// VM workspace virtiofs tag.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/vm.rs`: Virtiofs configuration
pub const CI_VM_WORKSPACE_TAG: &str = "workspace";

/// VM Nix store virtiofs tag.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/vm.rs`: Virtiofs configuration
pub const CI_VM_NIX_STORE_TAG: &str = "nix-store";

/// VM writable store overlay virtiofs tag.
///
/// Provides disk-backed storage for nix build artifacts inside CI VMs.
/// Without this, the tmpfs-backed overlay runs out of space during large builds.
///
/// Used in:
/// - `aspen-ci/workers/cloud_hypervisor/vm.rs`: Virtiofs configuration
pub const CI_VM_RW_STORE_TAG: &str = "rw-store";
