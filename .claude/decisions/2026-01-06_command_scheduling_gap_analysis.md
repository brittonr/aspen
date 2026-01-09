# Command Scheduling Gap Analysis

**Date**: 2026-01-06
**Analysis Type**: ULTRA Mode - Maximum Capability Analysis
**Question**: How far are we from being able to schedule a command (like goose-cli) on a schedule on an available worker node?

## Executive Summary

Aspen has **approximately 75-80% of the infrastructure** needed for distributed command scheduling. The core scheduling and worker coordination systems are production-ready. The main gap is a **ShellCommandWorker** implementation that can execute arbitrary shell commands with proper output handling, timeout management, and security controls.

**Estimated Development Time**: 1-2 weeks for minimal viable implementation, 3-4 weeks for production-ready with security hardening.

---

## Current State: What Already Exists

### 1. Durable Job Scheduling (95% Complete)

**Location**: `crates/aspen-jobs/src/scheduler.rs` (616 lines)

The `SchedulerService` provides:

- Sub-second precision scheduling (100ms tick interval)
- Full cron expression support via the `cron` crate
- Durable persistence to distributed KV store (survives restarts)
- Automatic recovery of up to 10,000 schedules on startup
- Multiple schedule types:
  - `Schedule::Once(DateTime<Utc>)` - one-time execution
  - `Schedule::Recurring(String)` - cron expressions
  - `Schedule::Interval { every, start_at }` - fixed intervals
  - `Schedule::RateLimit { max_per_hour }` - rate limiting
  - `Schedule::BusinessHours { days, start_hour, end_hour }` - business hours
  - `Schedule::Exponential { base_delay, max_delay }` - backoff

**Catch-up Policies** (for missed executions):

- `RunImmediately` - execute immediately when discovered
- `Skip` - skip to next scheduled time
- `RunAll` - run all missed instances
- `RunLatest` - run only the most recent missed instance

**Conflict Policies** (for overlapping executions):

- `Skip` - skip if previous still running
- `Queue` - queue behind running instance
- `Parallel` - allow parallel execution
- `Cancel` - cancel previous and start new

### 2. Worker Pool System (90% Complete)

**Location**: `crates/aspen-jobs/src/worker.rs` (526 lines)

The `WorkerPool` provides:

- Worker trait for custom job handlers
- Priority-based job queues (Critical, High, Normal, Low)
- Configurable concurrency per worker
- Heartbeat and health monitoring
- Visibility timeout for in-progress jobs
- Graceful shutdown with timeout

```rust
#[async_trait]
pub trait Worker: Send + Sync + 'static {
    async fn execute(&self, job: Job) -> JobResult;
    async fn on_start(&self) -> Result<()> { Ok(()) }
    async fn on_shutdown(&self) -> Result<()> { Ok(()) }
    fn job_types(&self) -> Vec<String> { vec![] }
    fn can_handle(&self, job_type: &str) -> bool { ... }
}
```

### 3. Distributed Worker Coordination (95% Complete)

**Location**: `crates/aspen-coordination/src/worker_coordinator.rs` (1228 lines)

The `DistributedWorkerCoordinator` provides:

- Global worker registry with health status
- Load balancing strategies:
  - `RoundRobin` - simple round-robin
  - `LeastLoaded` - route to least loaded worker
  - `Affinity` - route based on job affinity key
  - `ConsistentHash` - deterministic routing
  - `WorkStealing` - dynamic load rebalancing
- Worker groups for coordinated tasks
- Automatic failover and job redistribution
- Work stealing with steal hints in KV store
- Tiger Style limits (MAX_WORKERS = 1024, MAX_GROUPS = 64)

### 4. Job Management (95% Complete)

**Location**: `crates/aspen-jobs/src/manager.rs`

The `JobManager` provides:

- Job submission with multiple scheduling options
- Priority-based queue management
- Retry policies (None, Fixed, Exponential, Custom)
- Dead Letter Queue for permanently failed jobs
- Job dependency tracking
- Job cancellation and status tracking

### 5. Built-in Workers

**Location**: `crates/aspen-jobs/src/workers/`

Existing worker implementations:

- `EchoWorker` - test worker that echoes payloads
- `MaintenanceWorker` - system maintenance tasks
- `BlobProcessorWorker` - large value processing
- `ReplicationWorker` - data synchronization
- `SqlQueryWorker` - SQL query execution

### 6. VM Executor (Feature-Gated)

**Location**: `crates/aspen-jobs/src/vm_executor/` (requires `vm-executor` feature)

The `HyperlightWorker` provides sandboxed execution via micro-VMs:

- Supports pre-built binaries
- On-demand Nix builds
- Memory and CPU isolation

### 7. CLI for Job Management (90% Complete)

**Location**: `crates/aspen-cli/src/bin/aspen-cli/commands/job.rs` (857 lines)

Existing CLI commands:

- `aspen job submit <job_type> <payload> [--schedule <cron>] [--priority] [--timeout]`
- `aspen job submit-vm <binary> [--input]` - VM execution with binary upload
- `aspen job get <job_id>`
- `aspen job list [--status] [--job-type] [--tags]`
- `aspen job cancel <job_id>`
- `aspen job stats`
- `aspen job workers`
- `aspen job status <job_id> [--follow]`
- `aspen job result <job_id> [--timeout]`

---

## What's Missing: Gap Analysis

### Gap 1: ShellCommandWorker (CRITICAL - Does Not Exist)

**Effort**: 1-2 weeks

A dedicated worker for executing arbitrary shell commands is needed:

```rust
// Proposed implementation
pub struct ShellCommandWorker {
    allowed_commands: Option<HashSet<String>>,  // Allowlist for security
    default_shell: String,                       // "/bin/sh" or custom
    working_dir: PathBuf,                        // Default CWD
    env_filter: EnvFilter,                       // Environment variable filtering
    max_output_bytes: usize,                     // Output size limit
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommandPayload {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub timeout_ms: Option<u64>,
    pub capture_stderr: bool,
    pub stdin: Option<String>,
}
```

**Required capabilities**:

1. Parse command from job payload
2. Execute in isolated subprocess with signal handling
3. Capture stdout/stderr (bounded to prevent OOM)
4. Enforce timeout with SIGTERM -> SIGKILL escalation
5. Proper exit code propagation
6. Working directory management
7. Environment variable filtering (security)

### Gap 2: Output Handling for Large Results (MEDIUM)

**Effort**: 2-3 days

Current limitation: Job results are stored in KV with ~1MB limit.

**Solution**:

- For outputs > 1MB, automatically store in iroh-blobs
- Store blob hash in job result metadata
- Streaming output capture for very long commands

### Gap 3: CLI Command Scheduler (LOW)

**Effort**: 2-3 days

Add a dedicated subcommand for scheduling shell commands:

```bash
# Proposed CLI
aspen schedule add "goose-cli migrate" \
    --cron "0 2 * * *" \
    --tags "database" \
    --timeout 300s \
    --node-filter "capability=database"

aspen schedule list
aspen schedule pause <schedule_id>
aspen schedule resume <schedule_id>
aspen schedule delete <schedule_id>
aspen schedule next <schedule_id> --count 10  # Show next N executions
```

### Gap 4: Security Hardening (MEDIUM-HIGH)

**Effort**: 1 week

For production use, need:

1. Command allowlist validation
2. Sandboxing options (chroot, containers, or Hyperlight VM)
3. Resource limits (CPU, memory, disk)
4. Audit logging for command execution
5. Rate limiting per command type

---

## Implementation Roadmap

### Phase 1: Minimal Viable Implementation (1-2 weeks)

1. **Create `ShellCommandWorker`** in `crates/aspen-jobs/src/workers/shell_command.rs`
   - Basic command execution via `tokio::process::Command`
   - Timeout enforcement with SIGTERM/SIGKILL
   - Stdout/stderr capture
   - Exit code handling

2. **Add `job_type: "shell_command"` handler** registration in worker service

3. **Update CLI** to support shell command submission:

   ```bash
   aspen job submit shell_command '{"command":"goose-cli","args":["migrate"]}' \
       --schedule "0 2 * * *"
   ```

### Phase 2: Production Hardening (1-2 weeks additional)

1. **Command allowlist** in node configuration
2. **Large output handling** via iroh-blobs offload
3. **Dedicated `schedule` CLI subcommand** for better UX
4. **Environment variable filtering** for security
5. **Resource limits** via cgroups or Hyperlight

### Phase 3: Advanced Features (Optional)

1. **Command templates** - reusable command definitions
2. **Dependency chains** - command A runs after command B
3. **Output streaming** - live output via subscription
4. **Webhook notifications** - notify on completion/failure

---

## Architecture for Command Scheduling

```
User schedules "goose-cli migrate" daily at 2am
                    │
                    v
┌──────────────────────────────────────────┐
│ CLI: aspen schedule add                   │
│   --command "goose-cli migrate"           │
│   --cron "0 2 * * *"                      │
│   --tags "database"                       │
└────────────────────┬─────────────────────┘
                     │
                     v
┌──────────────────────────────────────────┐
│ SchedulerService                          │
│   - Parses cron: "0 2 * * *"              │
│   - Creates JobSpec with shell_command    │
│   - Persists to KV (survives restarts)    │
│   - Schedules next: 2026-01-07 02:00 UTC  │
└────────────────────┬─────────────────────┘
                     │
                     v
┌──────────────────────────────────────────┐
│ At 2am: SchedulerService.tick()           │
│   - Finds due schedules                   │
│   - Submits job to JobManager             │
└────────────────────┬─────────────────────┘
                     │
                     v
┌──────────────────────────────────────────┐
│ JobManager                                │
│   - Creates Job with ID                   │
│   - Enqueues to priority queue            │
│   - Routes based on tags/capabilities     │
└────────────────────┬─────────────────────┘
                     │
                     v
┌──────────────────────────────────────────┐
│ DistributedWorkerCoordinator              │
│   - Finds workers with "database" tag     │
│   - Selects least-loaded worker           │
│   - Routes job to selected node           │
└────────────────────┬─────────────────────┘
                     │
                     v
┌──────────────────────────────────────────┐
│ ShellCommandWorker (TO BE BUILT)          │
│   - Dequeues job                          │
│   - Validates command                     │
│   - Executes: goose-cli migrate           │
│   - Captures output                       │
│   - Handles timeout                       │
│   - Reports result                        │
└────────────────────┬─────────────────────┘
                     │
                     v
┌──────────────────────────────────────────┐
│ JobManager                                │
│   - Marks job completed/failed            │
│   - Stores result in KV                   │
│   - Updates metrics                       │
└──────────────────────────────────────────┘
```

---

## Risk Assessment

### Low Risk

- Schedule parsing and persistence (already working)
- Worker coordination (already working)
- Priority queues and retry logic (already working)

### Medium Risk

- Output size management (need blob offload)
- Timeout handling (need proper signal handling)
- Cross-platform compatibility (signal handling differs)

### High Risk

- Security (command injection, environment leakage)
- Resource exhaustion (runaway commands)
- Error handling for subprocess failures

---

## Comparison with External Systems

| Feature | Aspen (Current) | Nomad | Kubernetes CronJob |
|---------|-----------------|-------|-------------------|
| Cron scheduling | Yes | Yes | Yes |
| Durable persistence | Yes (Raft) | Yes (Raft) | Yes (etcd) |
| Worker discovery | Yes (gossip) | Yes | Yes (kubelet) |
| Load balancing | Yes (5 strategies) | Yes | Yes |
| Shell execution | **Missing** | Yes (exec driver) | Yes (containers) |
| Sandboxing | Partial (VM executor) | Yes (isolation) | Yes (containers) |
| Output streaming | No | Yes | Yes (logs) |

---

## Recommendation

**Start with Phase 1** - implement a basic `ShellCommandWorker` that:

1. Executes commands via `tokio::process::Command`
2. Captures stdout/stderr (bounded to 1MB)
3. Enforces timeouts with SIGTERM/SIGKILL
4. Returns exit code and output in job result

This gives you functional command scheduling in 1-2 weeks. Security hardening and advanced features can follow based on production needs.

---

## Key Files for Implementation

1. **New file**: `crates/aspen-jobs/src/workers/shell_command.rs` - ShellCommandWorker
2. **Modify**: `crates/aspen-jobs/src/workers/mod.rs` - Export ShellCommandWorker
3. **Modify**: `crates/aspen-cluster/src/worker_service.rs` - Register shell_command handler
4. **Modify**: `crates/aspen-cli/src/bin/aspen-cli/commands/job.rs` - Add schedule subcommand
5. **New file (optional)**: `crates/aspen-cli/src/bin/aspen-cli/commands/schedule.rs` - Dedicated schedule CLI
