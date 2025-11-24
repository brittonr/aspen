# Distributed Orchestrator Design for mvm-ci

## Vision Statement

Transform mvm-ci from a **distributed work queue** into a **Kubernetes-like orchestrator** capable of managing heterogeneous worker types (Firecracker MicroVMs, WASM, containers) with intelligent scheduling, resource management, and auto-scaling.

## Executive Summary

**Current State:** Pull-based work queue with WASM execution backend
**Target State:** Declarative orchestrator with multiple worker types, resource-aware scheduling, and lifecycle management
**Timeline:** 6 phases over 16-20 weeks
**Key Differentiator:** P2P networking (iroh+h3) enables orchestration across NAT boundaries without static IPs

---

## Architecture Overview

### Current Architecture (Work Queue)

```
┌─────────────────────────────────────────────────┐
│           Control Plane (mvm-ci)                 │
│  ┌──────────────┐        ┌──────────────────┐  │
│  │   Hiqlite    │◄───────┤  Work Queue      │  │
│  │   (Raft DB)  │        │  (FCFS Claim)    │  │
│  └──────────────┘        └──────────────────┘  │
│                                 ▲                │
└─────────────────────────────────┼────────────────┘
                                  │ iroh+h3 P2P
                    ┌─────────────┴─────────────┐
                    │                           │
            ┌───────▼──────┐           ┌───────▼──────┐
            │   Worker 1   │           │   Worker 2   │
            │ (FlawlessWASM)│          │ (FlawlessWASM)│
            └──────────────┘           └──────────────┘
                  Pull                      Pull
            (claim_work every 2s)    (claim_work every 2s)
```

**Issues:**
- Workers are anonymous (no registration)
- No resource tracking (CPU/memory blind)
- FCFS scheduling (no priorities or affinity)
- Polling wastes bandwidth when idle
- Workers don't report health/capacity

### Target Architecture (Orchestrator)

```
┌─────────────────────────────────────────────────────────────────┐
│                  Control Plane (Orchestrator)                    │
│                                                                   │
│  ┌────────────────┐  ┌──────────────┐  ┌─────────────────────┐ │
│  │ Worker Registry│  │  Scheduler    │  │  Lifecycle Manager  │ │
│  │ - Heartbeats   │  │ - Bin-pack    │  │  - Health checks    │ │
│  │ - Capacity     │  │ - Affinity    │  │  - Auto-scale       │ │
│  │ - Labels       │  │ - Priorities  │  │  - Restart policy   │ │
│  └───────┬────────┘  └──────┬───────┘  └──────────┬──────────┘ │
│          │                  │                      │             │
│          └──────────────────┼──────────────────────┘             │
│                             ▼                                     │
│                    ┌─────────────────┐                           │
│                    │   Hiqlite (DB)  │                           │
│                    │  - Jobs         │                           │
│                    │  - Workers      │                           │
│                    │  - Assignments  │                           │
│                    └─────────────────┘                           │
└─────────────────────────────────────────────────────────────────┘
                             │ iroh+h3 P2P
            ┌────────────────┼────────────────┬───────────────┐
            │                │                │               │
    ┌───────▼──────┐  ┌─────▼──────┐  ┌─────▼──────┐  ┌────▼──────┐
    │ Firecracker  │  │   WASM     │  │  Container │  │   Custom  │
    │   Worker     │  │   Worker   │  │   Worker   │  │   Worker  │
    │              │  │            │  │            │  │           │
    │ • Registers  │  │ • Registers│  │ • Registers│  │• Registers│
    │ • Heartbeats │  │ • Reports  │  │ • Reports  │  │• Reports  │
    │ • Reports    │  │   capacity │  │   capacity │  │  capacity │
    │   capacity   │  │ • Executes │  │ • Executes │  │• Executes │
    └──────────────┘  └────────────┘  └────────────┘  └───────────┘
```

**Key Improvements:**
- ✅ Workers register with capabilities (CPU, RAM, labels)
- ✅ Resource-aware scheduling (match jobs to capacity)
- ✅ Smart placement (affinity, priorities, constraints)
- ✅ Health monitoring (heartbeats, liveness probes)
- ✅ Auto-scaling (spawn/destroy workers dynamically)
- ✅ Heterogeneous worker types (MicroVMs, WASM, containers)

---

## Core Components

### 1. Worker Registry

**Purpose:** Track all workers, their capabilities, health, and resource utilization

**Database Schema:**
```sql
CREATE TABLE workers (
    node_id TEXT PRIMARY KEY,                -- iroh endpoint ID
    worker_type TEXT NOT NULL,               -- 'firecracker', 'wasm', 'container'
    status TEXT NOT NULL,                    -- 'online', 'offline', 'draining'
    registered_at INTEGER NOT NULL,
    last_heartbeat INTEGER NOT NULL,

    -- Resource capacity
    cpu_millicores INTEGER NOT NULL,         -- 1000 = 1 CPU core
    memory_mb INTEGER NOT NULL,
    disk_mb INTEGER,
    gpu_count INTEGER DEFAULT 0,

    -- Resource utilization (updated by heartbeats)
    allocated_cpu_millicores INTEGER DEFAULT 0,
    allocated_memory_mb INTEGER DEFAULT 0,

    -- Labels for scheduling (JSON)
    labels TEXT,                             -- {"region": "us-west", "hardware": "amd"}

    -- Health
    health_status TEXT,                      -- 'healthy', 'unhealthy', 'unknown'
    health_message TEXT,

    -- Version tracking
    software_version TEXT
);

CREATE INDEX idx_workers_status ON workers(status);
CREATE INDEX idx_workers_last_heartbeat ON workers(last_heartbeat);
```

**API Endpoints:**

```rust
// Worker registers on startup
POST /api/workers/register
{
    "node_id": "abc123...",
    "worker_type": "firecracker",
    "cpu_millicores": 4000,      // 4 CPU cores
    "memory_mb": 8192,           // 8 GB RAM
    "disk_mb": 20480,            // 20 GB disk
    "gpu_count": 1,
    "labels": {
        "region": "us-west-2",
        "instance_type": "c5.xlarge",
        "hardware": "intel"
    },
    "software_version": "0.2.0"
}

// Worker sends heartbeat every 10 seconds
POST /api/workers/{node_id}/heartbeat
{
    "allocated_cpu_millicores": 1500,
    "allocated_memory_mb": 2048,
    "health_status": "healthy",
    "active_jobs": ["job-123", "job-456"]
}

// Control plane queries
GET /api/workers                              // List all workers
GET /api/workers/{node_id}                    // Get worker details
POST /api/workers/{node_id}/drain             // Mark for graceful shutdown
DELETE /api/workers/{node_id}                 // Deregister worker
```

**Worker Client Changes (`src/bin/worker.rs`):**

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    let control_plane_ticket = std::env::var("CONTROL_PLANE_TICKET")?;

    // Connect to control plane
    let client = WorkQueueClient::connect(&control_plane_ticket).await?;

    // NEW: Register worker
    let registration = WorkerRegistration {
        node_id: client.node_id().to_string(),
        worker_type: WorkerType::Firecracker, // or WASM, Container
        cpu_millicores: 4000,
        memory_mb: 8192,
        disk_mb: Some(20480),
        gpu_count: 0,
        labels: get_worker_labels(), // From env or config
        software_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    client.register(registration).await?;
    tracing::info!("Worker registered successfully");

    // NEW: Spawn heartbeat task
    let heartbeat_client = client.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(e) = heartbeat_client.send_heartbeat().await {
                tracing::error!("Heartbeat failed: {}", e);
            }
        }
    });

    // Initialize backend
    let worker = create_worker_backend(registration.worker_type, &config).await?;
    worker.initialize().await?;

    // Main worker loop (unchanged)
    loop {
        match client.claim_work().await {
            Ok(Some(job)) => { /* ... */ }
            Ok(None) => tokio::time::sleep(Duration::from_secs(2)).await,
            Err(e) => { /* ... */ }
        }
    }
}
```

**Implementation:**
- New file: `src/domain/worker_registry.rs`
- New handler: `src/handlers/workers.rs`
- New repository: `src/repositories/worker_repository.rs`

---

### 2. Resource Management

**Purpose:** Track resource requirements and prevent overcommitment

**Job Schema Extension:**
```sql
CREATE TABLE workflows (
    -- ... existing fields ...

    -- NEW: Resource requirements
    required_cpu_millicores INTEGER DEFAULT 100,
    required_memory_mb INTEGER DEFAULT 128,
    required_disk_mb INTEGER DEFAULT 0,
    required_gpu_count INTEGER DEFAULT 0,

    -- NEW: Scheduling constraints
    scheduling_policy TEXT,      -- JSON: priorities, affinity, tolerations
    assigned_worker TEXT,         -- node_id of assigned worker (nullable)

    FOREIGN KEY (assigned_worker) REFERENCES workers(node_id)
);
```

**Domain Types:**

```rust
// src/domain/types.rs
pub struct Job {
    pub id: String,
    pub status: JobStatus,
    pub payload: serde_json::Value,

    // NEW: Resource requirements
    pub resource_requirements: ResourceRequirements,

    // NEW: Scheduling policy
    pub scheduling_policy: SchedulingPolicy,

    // NEW: Assignment tracking
    pub assigned_worker: Option<String>,
    pub claimed_by: Option<String>,
    pub completed_by: Option<String>,

    // ... existing timestamp fields ...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_millicores: u32,
    pub memory_mb: u32,
    pub disk_mb: u32,
    pub gpu_count: u32,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_millicores: 100,   // 0.1 CPU
            memory_mb: 128,        // 128 MB
            disk_mb: 0,
            gpu_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingPolicy {
    pub priority: JobPriority,
    pub affinity: Option<AffinityRules>,
    pub tolerations: Vec<Toleration>,
    pub preemptible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Critical = 4,    // Preempts others
    High = 3,
    Normal = 2,
    Low = 1,
    BestEffort = 0,  // Can be preempted anytime
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityRules {
    // REQUIRED: Job fails if no matching worker
    pub node_selector: HashMap<String, String>,

    // PREFERRED: Soft preference, scheduler tries to satisfy
    pub preferred_nodes: Vec<String>,

    // ANTI-AFFINITY: Avoid workers already running these jobs
    pub anti_affinity_job_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toleration {
    // Allow scheduling on workers with specific taints
    pub key: String,
    pub operator: TolerationType,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TolerationType {
    Equal,     // Exact match
    Exists,    // Just check key exists
}
```

**Job Submission API:**

```rust
// POST /api/queue/publish
{
    "payload": {
        "url": "https://example.com",
        "id": 123
    },
    "resource_requirements": {
        "cpu_millicores": 500,    // 0.5 CPU
        "memory_mb": 1024,        // 1 GB
        "disk_mb": 2048,          // 2 GB ephemeral
        "gpu_count": 0
    },
    "scheduling_policy": {
        "priority": "High",
        "affinity": {
            "node_selector": {
                "region": "us-west-2",
                "hardware": "intel"
            },
            "preferred_nodes": ["worker-abc", "worker-xyz"]
        },
        "tolerations": [
            {
                "key": "workload-type",
                "operator": "Equal",
                "value": "compute-intensive"
            }
        ],
        "preemptible": false
    }
}
```

---

### 3. Smart Scheduler

**Purpose:** Intelligently place jobs on workers based on resources, affinity, and priorities

**Algorithm: Best-Fit Bin Packing**

```rust
// src/scheduler/mod.rs
pub trait Scheduler: Send + Sync {
    /// Find the best worker for a job
    async fn schedule(&self, job: &Job) -> Result<Option<String>>;

    /// Attempt to schedule all pending jobs
    async fn schedule_all(&self) -> Result<Vec<(String, String)>>; // Vec<(job_id, worker_id)>

    /// Handle preemption if high-priority job can't be scheduled
    async fn preempt(&self, job: &Job) -> Result<Option<Vec<String>>>; // Preempted job IDs
}

// src/scheduler/bin_pack.rs
pub struct BinPackScheduler {
    worker_registry: Arc<dyn WorkerRepository>,
    work_repo: Arc<dyn WorkRepository>,
}

impl Scheduler for BinPackScheduler {
    async fn schedule(&self, job: &Job) -> Result<Option<String>> {
        // 1. Get all online workers
        let workers = self.worker_registry.list_online_workers().await?;

        // 2. Filter by hard constraints (node_selector, taints/tolerations)
        let eligible_workers = workers.into_iter()
            .filter(|w| self.matches_affinity(w, &job.scheduling_policy))
            .collect::<Vec<_>>();

        if eligible_workers.is_empty() {
            return Ok(None); // No eligible workers
        }

        // 3. Filter by resource availability
        let capable_workers = eligible_workers.into_iter()
            .filter(|w| self.has_capacity(w, &job.resource_requirements))
            .collect::<Vec<_>>();

        if capable_workers.is_empty() {
            // Check if we should preempt lower-priority jobs
            if job.scheduling_policy.priority >= JobPriority::High {
                return self.find_preemption_target(job, &eligible_workers).await;
            }
            return Ok(None);
        }

        // 4. Score workers (prefer less fragmented)
        let scored_workers = capable_workers.into_iter()
            .map(|w| {
                let score = self.calculate_score(&w, job);
                (w, score)
            })
            .collect::<Vec<_>>();

        // 5. Sort by score (prefer workers with tighter fit)
        let mut sorted = scored_workers;
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // 6. Return best worker
        Ok(sorted.first().map(|(w, _)| w.node_id.clone()))
    }
}

impl BinPackScheduler {
    fn matches_affinity(&self, worker: &Worker, policy: &SchedulingPolicy) -> bool {
        if let Some(affinity) = &policy.affinity {
            // Check node_selector (hard constraint)
            for (key, value) in &affinity.node_selector {
                if worker.labels.get(key) != Some(value) {
                    return false; // Missing required label
                }
            }
        }

        // Check tolerations (can worker accept this job?)
        for toleration in &policy.tolerations {
            if !self.worker_has_taint(worker, toleration) {
                return false;
            }
        }

        true
    }

    fn has_capacity(&self, worker: &Worker, requirements: &ResourceRequirements) -> bool {
        let available_cpu = worker.cpu_millicores - worker.allocated_cpu_millicores;
        let available_memory = worker.memory_mb - worker.allocated_memory_mb;

        available_cpu >= requirements.cpu_millicores
            && available_memory >= requirements.memory_mb
            && worker.gpu_count >= requirements.gpu_count
    }

    fn calculate_score(&self, worker: &Worker, job: &Job) -> f64 {
        // Best-fit: prefer worker with least remaining resources after allocation
        // (minimizes fragmentation)

        let cpu_after = worker.cpu_millicores - worker.allocated_cpu_millicores
            - job.resource_requirements.cpu_millicores;
        let mem_after = worker.memory_mb - worker.allocated_memory_mb
            - job.resource_requirements.memory_mb;

        // Lower remaining resources = higher score (tighter fit)
        let cpu_score = 1.0 - (cpu_after as f64 / worker.cpu_millicores as f64);
        let mem_score = 1.0 - (mem_after as f64 / worker.memory_mb as f64);

        // Weighted average (70% CPU, 30% memory)
        cpu_score * 0.7 + mem_score * 0.3
    }

    async fn find_preemption_target(
        &self,
        high_priority_job: &Job,
        workers: &[Worker],
    ) -> Result<Option<String>> {
        // Find worker where we can evict lower-priority jobs to make room
        for worker in workers {
            let running_jobs = self.work_repo
                .list_jobs_on_worker(&worker.node_id)
                .await?;

            let preemptible_jobs = running_jobs.into_iter()
                .filter(|j| j.scheduling_policy.priority < high_priority_job.scheduling_policy.priority)
                .filter(|j| j.scheduling_policy.preemptible)
                .collect::<Vec<_>>();

            // Calculate freed resources
            let freed_cpu: u32 = preemptible_jobs.iter()
                .map(|j| j.resource_requirements.cpu_millicores)
                .sum();
            let freed_memory: u32 = preemptible_jobs.iter()
                .map(|j| j.resource_requirements.memory_mb)
                .sum();

            let available_cpu = worker.cpu_millicores - worker.allocated_cpu_millicores + freed_cpu;
            let available_memory = worker.memory_mb - worker.allocated_memory_mb + freed_memory;

            if available_cpu >= high_priority_job.resource_requirements.cpu_millicores
                && available_memory >= high_priority_job.resource_requirements.memory_mb
            {
                // Preempt these jobs
                for job in preemptible_jobs {
                    self.work_repo.update_status(
                        &job.id,
                        JobStatus::Pending,
                        Some("Preempted by higher priority job".to_string()),
                    ).await?;
                }

                return Ok(Some(worker.node_id.clone()));
            }
        }

        Ok(None)
    }
}
```

**Scheduler Integration:**

```rust
// src/domain/job_lifecycle.rs
pub struct JobLifecycleService {
    work_repo: Arc<dyn WorkRepository>,
    scheduler: Arc<dyn Scheduler>,  // NEW
}

impl JobLifecycleService {
    pub async fn submit_job(&self, submission: JobSubmission) -> Result<String> {
        // ... create job ...

        // Persist as pending
        self.work_repo.publish_work(job_id.clone(), job).await?;

        // NEW: Immediately try to schedule
        if let Some(worker_id) = self.scheduler.schedule(&job).await? {
            self.work_repo.assign_to_worker(&job_id, &worker_id).await?;
            tracing::info!(
                job_id = %job_id,
                worker_id = %worker_id,
                "Job scheduled successfully"
            );
        } else {
            tracing::warn!(
                job_id = %job_id,
                "Job submitted but no eligible workers available"
            );
        }

        Ok(job_id)
    }
}
```

**Background Scheduler Loop:**

```rust
// src/main.rs
async fn start_scheduler_loop(
    scheduler: Arc<dyn Scheduler>,
    interval_secs: u64,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    loop {
        interval.tick().await;

        match scheduler.schedule_all().await {
            Ok(assignments) => {
                tracing::info!(
                    assignments_count = assignments.len(),
                    "Scheduler run completed"
                );
            }
            Err(e) => {
                tracing::error!(error = %e, "Scheduler run failed");
            }
        }
    }
}

// In main():
tokio::spawn(start_scheduler_loop(
    state.scheduler().clone(),
    5, // Run every 5 seconds
));
```

---

### 4. Worker Lifecycle Management

**Purpose:** Manage worker health, restarts, and graceful shutdown

**Health Monitoring:**

```rust
// src/lifecycle/health_monitor.rs
pub struct HealthMonitor {
    worker_registry: Arc<dyn WorkerRepository>,
    work_repo: Arc<dyn WorkRepository>,
    stale_threshold_secs: u64,
}

impl HealthMonitor {
    pub async fn check_all_workers(&self) -> Result<()> {
        let workers = self.worker_registry.list_all_workers().await?;
        let now = chrono::Utc::now().timestamp();

        for worker in workers {
            let seconds_since_heartbeat = now - worker.last_heartbeat;

            if seconds_since_heartbeat > self.stale_threshold_secs as i64 {
                // Worker is stale
                tracing::warn!(
                    node_id = %worker.node_id,
                    seconds = seconds_since_heartbeat,
                    "Worker missed heartbeat, marking offline"
                );

                // Mark worker offline
                self.worker_registry
                    .update_status(&worker.node_id, WorkerStatus::Offline)
                    .await?;

                // Requeue jobs that were running on this worker
                let orphaned_jobs = self.work_repo
                    .list_jobs_on_worker(&worker.node_id)
                    .await?;

                for job in orphaned_jobs {
                    if job.status == JobStatus::InProgress || job.status == JobStatus::Claimed {
                        self.work_repo.update_status(
                            &job.id,
                            JobStatus::Pending,
                            Some("Worker went offline".to_string()),
                        ).await?;

                        tracing::info!(
                            job_id = %job.id,
                            "Requeued job from offline worker"
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

// Background task in main.rs:
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(15));
    loop {
        interval.tick().await;
        if let Err(e) = health_monitor.check_all_workers().await {
            tracing::error!(error = %e, "Health check failed");
        }
    }
});
```

**Graceful Shutdown:**

```rust
// src/bin/worker.rs
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // ... registration and heartbeat setup ...

    // NEW: Setup graceful shutdown
    let shutdown_signal = async {
        let _ = signal::ctrl_c().await;
        tracing::info!("Received SIGINT, initiating graceful shutdown");
    };

    tokio::select! {
        _ = shutdown_signal => {
            tracing::info!("Shutting down worker gracefully");

            // Notify control plane we're draining
            client.mark_draining().await?;

            // Wait for active jobs to finish (with timeout)
            let shutdown_timeout = Duration::from_secs(300); // 5 minutes
            tokio::select! {
                _ = wait_for_active_jobs_to_finish() => {
                    tracing::info!("All jobs finished, shutting down");
                }
                _ = tokio::time::sleep(shutdown_timeout) => {
                    tracing::warn!("Shutdown timeout reached, forcing exit");
                }
            }

            // Cleanup
            worker.shutdown().await?;
            client.deregister().await?;
        }

        result = run_worker_loop(client, worker) => {
            result?;
        }
    }

    Ok(())
}

async fn wait_for_active_jobs_to_finish() {
    // Poll until no active jobs
    loop {
        if ACTIVE_JOBS.load(Ordering::SeqCst) == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
```

**Restart Policies:**

```rust
// src/domain/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartPolicy {
    pub restart_on_failure: bool,
    pub max_retries: u32,
    pub backoff_strategy: BackoffStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Immediate,
    Linear { delay_secs: u64 },
    Exponential { base_delay_secs: u64, max_delay_secs: u64 },
}

// src/lifecycle/restart_manager.rs
pub struct RestartManager {
    work_repo: Arc<dyn WorkRepository>,
}

impl RestartManager {
    pub async fn process_failed_jobs(&self) -> Result<()> {
        let failed_jobs = self.work_repo
            .list_jobs_by_status(JobStatus::Failed)
            .await?;

        for job in failed_jobs {
            if job.retry_count >= job.restart_policy.max_retries {
                tracing::info!(
                    job_id = %job.id,
                    retries = job.retry_count,
                    "Job exhausted retries, giving up"
                );
                continue;
            }

            // Calculate backoff delay
            let delay = match job.restart_policy.backoff_strategy {
                BackoffStrategy::Immediate => Duration::from_secs(0),
                BackoffStrategy::Linear { delay_secs } => {
                    Duration::from_secs(delay_secs * job.retry_count as u64)
                }
                BackoffStrategy::Exponential { base_delay_secs, max_delay_secs } => {
                    let delay = base_delay_secs * 2u64.pow(job.retry_count);
                    Duration::from_secs(delay.min(max_delay_secs))
                }
            };

            // Schedule retry
            tokio::time::sleep(delay).await;

            self.work_repo.update_status(
                &job.id,
                JobStatus::Pending,
                Some(format!("Retry {}/{}", job.retry_count + 1, job.restart_policy.max_retries)),
            ).await?;

            self.work_repo.increment_retry_count(&job.id).await?;

            tracing::info!(
                job_id = %job.id,
                retry = job.retry_count + 1,
                delay_secs = delay.as_secs(),
                "Retrying failed job"
            );
        }

        Ok(())
    }
}
```

---

### 5. Heterogeneous Worker Types

**Purpose:** Support multiple backend types (Firecracker, WASM, containers) with different capabilities

**Worker Type System:**

```rust
// src/domain/types.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkerType {
    Flawless,      // WASM execution via Flawless
    Firecracker,   // MicroVM via Firecracker
    Container,     // Docker/Podman container
    Custom(String),// Extensible for future types
}

// src/worker_firecracker.rs
pub struct FirecrackerWorker {
    firecracker_client: FirecrackerClient,
    vm_template: VmTemplate,
}

impl FirecrackerWorker {
    pub async fn new(config: &FirecrackerConfig) -> Result<Self> {
        let client = FirecrackerClient::new(config.socket_path.clone());
        let template = VmTemplate::load(&config.template_path)?;

        Ok(Self {
            firecracker_client: client,
            vm_template: template,
        })
    }
}

#[async_trait]
impl WorkerBackend for FirecrackerWorker {
    async fn execute(&self, job: Job) -> Result<WorkResult> {
        // 1. Create VM instance from template
        let vm_config = VmConfig {
            vcpu_count: (job.resource_requirements.cpu_millicores / 1000).max(1),
            mem_size_mib: job.resource_requirements.memory_mb,
            kernel_image_path: self.vm_template.kernel_path.clone(),
            rootfs_path: self.vm_template.rootfs_path.clone(),
        };

        let vm_id = format!("job-{}", job.id);

        tracing::info!(vm_id = %vm_id, "Starting Firecracker VM");

        // 2. Boot VM
        self.firecracker_client
            .create_vm(&vm_id, vm_config)
            .await?;

        // 3. Wait for VM to be ready
        self.firecracker_client.wait_for_ready(&vm_id).await?;

        // 4. Execute job payload in VM
        let result = self.execute_in_vm(&vm_id, &job.payload).await;

        // 5. Cleanup VM
        self.firecracker_client.destroy_vm(&vm_id).await?;

        match result {
            Ok(output) => Ok(WorkResult::success_with_output(output)),
            Err(e) => Ok(WorkResult::failure(e.to_string())),
        }
    }
}

// src/bin/worker.rs
fn create_worker_backend(
    worker_type: WorkerType,
    config: &AppConfig,
) -> Result<Box<dyn WorkerBackend>> {
    match worker_type {
        WorkerType::Flawless => {
            let worker = FlawlessWorker::new(&config.flawless.flawless_url).await?;
            Ok(Box::new(worker))
        }
        WorkerType::Firecracker => {
            let worker = FirecrackerWorker::new(&config.firecracker).await?;
            Ok(Box::new(worker))
        }
        WorkerType::Container => {
            let worker = ContainerWorker::new(&config.container).await?;
            Ok(Box::new(worker))
        }
        WorkerType::Custom(name) => {
            anyhow::bail!("Unsupported worker type: {}", name);
        }
    }
}
```

**Job Type Matching:**

```rust
// src/domain/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    // ... existing fields ...

    // NEW: Specify which worker types can run this job
    pub compatible_worker_types: Vec<WorkerType>,
}

// src/scheduler/bin_pack.rs
impl BinPackScheduler {
    fn matches_worker_type(&self, worker: &Worker, job: &Job) -> bool {
        if job.compatible_worker_types.is_empty() {
            return true; // No restriction
        }

        job.compatible_worker_types.contains(&worker.worker_type)
    }
}
```

**Configuration:**

```rust
// src/config.rs
#[derive(Debug, Clone)]
pub struct FirecrackerConfig {
    pub socket_path: PathBuf,
    pub kernel_path: PathBuf,
    pub rootfs_path: PathBuf,
    pub template_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub docker_socket: String,
    pub default_image: String,
    pub network_mode: String,
}

pub struct AppConfig {
    // ... existing fields ...
    pub firecracker: FirecrackerConfig,
    pub container: ContainerConfig,
}
```

---

### 6. Auto-Scaling

**Purpose:** Dynamically adjust worker pool size based on demand

**Auto-Scaler Design:**

```rust
// src/autoscaler/mod.rs
pub struct AutoScaler {
    worker_registry: Arc<dyn WorkerRepository>,
    work_repo: Arc<dyn WorkRepository>,
    config: AutoScalerConfig,
    vm_provisioner: Arc<dyn VmProvisioner>,
}

#[derive(Debug, Clone)]
pub struct AutoScalerConfig {
    pub enabled: bool,
    pub min_workers: usize,
    pub max_workers: usize,
    pub target_cpu_percent: f64,
    pub scale_up_threshold: f64,      // Scale up if queue depth > threshold
    pub scale_down_threshold: f64,
    pub cooldown_secs: u64,           // Wait between scaling actions
}

pub trait VmProvisioner: Send + Sync {
    /// Provision a new worker (VM, container, etc.)
    async fn provision_worker(&self, worker_type: WorkerType) -> Result<String>; // Returns node_id

    /// Deprovision a worker
    async fn deprovision_worker(&self, node_id: &str) -> Result<()>;
}

impl AutoScaler {
    pub async fn evaluate(&self) -> Result<ScalingDecision> {
        if !self.config.enabled {
            return Ok(ScalingDecision::NoAction);
        }

        // Get current metrics
        let workers = self.worker_registry.list_online_workers().await?;
        let pending_jobs = self.work_repo.list_jobs_by_status(JobStatus::Pending).await?;

        let current_worker_count = workers.len();
        let queue_depth = pending_jobs.len();

        // Calculate average CPU utilization
        let total_cpu_used: u32 = workers.iter()
            .map(|w| w.allocated_cpu_millicores)
            .sum();
        let total_cpu_capacity: u32 = workers.iter()
            .map(|w| w.cpu_millicores)
            .sum();

        let cpu_utilization = if total_cpu_capacity > 0 {
            total_cpu_used as f64 / total_cpu_capacity as f64
        } else {
            0.0
        };

        tracing::info!(
            workers = current_worker_count,
            queue_depth = queue_depth,
            cpu_utilization = %format!("{:.1}%", cpu_utilization * 100.0),
            "Auto-scaler evaluation"
        );

        // Scale up if:
        // - Queue is growing AND CPU is high
        // - OR queue depth exceeds threshold
        if current_worker_count < self.config.max_workers {
            if cpu_utilization > self.config.target_cpu_percent
                || queue_depth as f64 > self.config.scale_up_threshold
            {
                return Ok(ScalingDecision::ScaleUp(1));
            }
        }

        // Scale down if:
        // - CPU is low AND queue is empty
        if current_worker_count > self.config.min_workers {
            if cpu_utilization < 0.3 && queue_depth == 0 {
                return Ok(ScalingDecision::ScaleDown(1));
            }
        }

        Ok(ScalingDecision::NoAction)
    }

    pub async fn execute_scaling(&self, decision: ScalingDecision) -> Result<()> {
        match decision {
            ScalingDecision::ScaleUp(count) => {
                for _ in 0..count {
                    tracing::info!("Provisioning new worker");
                    let node_id = self.vm_provisioner
                        .provision_worker(WorkerType::Firecracker)
                        .await?;
                    tracing::info!(node_id = %node_id, "Worker provisioned successfully");
                }
            }
            ScalingDecision::ScaleDown(count) => {
                let workers = self.worker_registry.list_online_workers().await?;

                // Find idle workers (no active jobs)
                let idle_workers: Vec<_> = workers.into_iter()
                    .filter(|w| w.allocated_cpu_millicores == 0)
                    .take(count)
                    .collect();

                for worker in idle_workers {
                    tracing::info!(node_id = %worker.node_id, "Draining and deprovisioning worker");

                    // Mark as draining
                    self.worker_registry
                        .update_status(&worker.node_id, WorkerStatus::Draining)
                        .await?;

                    // Deprovision after grace period
                    self.vm_provisioner
                        .deprovision_worker(&worker.node_id)
                        .await?;
                }
            }
            ScalingDecision::NoAction => {}
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ScalingDecision {
    ScaleUp(usize),
    ScaleDown(usize),
    NoAction,
}

// Background task:
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    let mut last_scaling_action = Instant::now();

    loop {
        interval.tick().await;

        // Respect cooldown period
        if last_scaling_action.elapsed() < Duration::from_secs(autoscaler.config.cooldown_secs) {
            continue;
        }

        match autoscaler.evaluate().await {
            Ok(decision) => {
                if !matches!(decision, ScalingDecision::NoAction) {
                    if let Err(e) = autoscaler.execute_scaling(decision).await {
                        tracing::error!(error = %e, "Scaling action failed");
                    } else {
                        last_scaling_action = Instant::now();
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Auto-scaler evaluation failed");
            }
        }
    }
});
```

**VM Provisioner Implementation (Firecracker):**

```rust
// src/autoscaler/firecracker_provisioner.rs
pub struct FirecrackerProvisioner {
    config: FirecrackerConfig,
    control_plane_ticket: String,
}

#[async_trait]
impl VmProvisioner for FirecrackerProvisioner {
    async fn provision_worker(&self, worker_type: WorkerType) -> Result<String> {
        // 1. Generate unique VM ID
        let vm_id = format!("worker-{}", uuid::Uuid::new_v4());

        // 2. Create Firecracker VM
        let vm_config = VmConfig {
            vcpu_count: 2,
            mem_size_mib: 2048,
            kernel_image_path: self.config.kernel_path.clone(),
            rootfs_path: self.config.rootfs_path.clone(),
        };

        let firecracker = FirecrackerClient::new(&self.config.socket_path);
        firecracker.create_vm(&vm_id, vm_config).await?;

        // 3. Boot VM and wait for network
        firecracker.boot_vm(&vm_id).await?;
        let vm_ip = firecracker.get_vm_ip(&vm_id).await?;

        // 4. Start worker binary in VM via SSH
        let ssh_cmd = format!(
            "CONTROL_PLANE_TICKET={} /usr/bin/worker",
            self.control_plane_ticket
        );

        Command::new("ssh")
            .args(&[&format!("root@{}", vm_ip), &ssh_cmd])
            .spawn()?;

        // 5. Worker will register itself via /api/workers/register
        Ok(vm_id)
    }

    async fn deprovision_worker(&self, node_id: &str) -> Result<()> {
        let firecracker = FirecrackerClient::new(&self.config.socket_path);
        firecracker.destroy_vm(node_id).await?;
        Ok(())
    }
}
```

---

## Implementation Phases

### Phase 1: Worker Registry (4 weeks)
**Goal:** Explicit worker tracking with heartbeats

**Tasks:**
- [ ] Add `workers` table to hiqlite schema
- [ ] Implement `WorkerRepository` trait and Hiqlite impl
- [ ] Add `/api/workers/register` endpoint
- [ ] Add `/api/workers/{id}/heartbeat` endpoint
- [ ] Modify `src/bin/worker.rs` to register on startup
- [ ] Add heartbeat background task to worker
- [ ] Create health monitor background task
- [ ] Add dashboard view for workers (`/dashboard/workers`)

**Deliverables:**
- Workers visible in dashboard with live status
- Stale workers automatically marked offline
- Orphaned jobs requeued when worker dies

---

### Phase 2: Resource Management (3 weeks)
**Goal:** Track resource requirements and capacity

**Tasks:**
- [ ] Extend `Job` type with `ResourceRequirements`
- [ ] Extend `workflows` table schema
- [ ] Extend worker registration with capacity
- [ ] Add resource accounting to heartbeat
- [ ] Update job submission API to accept requirements
- [ ] Add validation (reject jobs exceeding cluster capacity)
- [ ] Dashboard shows resource utilization

**Deliverables:**
- Jobs specify CPU/memory/disk requirements
- Workers report capacity and utilization
- Control plane tracks allocated vs available resources

---

### Phase 3: Smart Scheduler (4 weeks)
**Goal:** Intelligent job placement

**Tasks:**
- [ ] Implement `Scheduler` trait
- [ ] Implement `BinPackScheduler` with best-fit algorithm
- [ ] Add affinity/anti-affinity rules to `SchedulingPolicy`
- [ ] Add priority levels to jobs
- [ ] Integrate scheduler into job submission
- [ ] Add background scheduler loop
- [ ] Implement preemption for high-priority jobs
- [ ] Add scheduler metrics (placement success rate, time)

**Deliverables:**
- Jobs placed on best-fit workers
- Affinity rules respected (node selectors)
- High-priority jobs can preempt low-priority ones

---

### Phase 4: Lifecycle Management (3 weeks)
**Goal:** Health checks, restarts, graceful shutdown

**Tasks:**
- [ ] Add restart policies to jobs
- [ ] Implement `RestartManager` with exponential backoff
- [ ] Add SIGTERM handling to worker binary
- [ ] Implement graceful drain protocol
- [ ] Add liveness probes (optional HTTP endpoint on workers)
- [ ] Add `/api/workers/{id}/drain` endpoint
- [ ] Background task for restart processing

**Deliverables:**
- Failed jobs retry with backoff
- Workers shutdown gracefully
- No job loss during worker restarts

---

### Phase 5: Heterogeneous Workers (4 weeks)
**Goal:** Support Firecracker MicroVMs

**Tasks:**
- [ ] Add `WorkerType` enum to domain
- [ ] Implement `FirecrackerWorker` backend
- [ ] Create Firecracker VM template (Nix-built rootfs)
- [ ] Add Firecracker client library
- [ ] Add `compatible_worker_types` to jobs
- [ ] Modify scheduler to respect worker type constraints
- [ ] Create worker startup scripts for VMs
- [ ] Test end-to-end Firecracker execution

**Deliverables:**
- Workers can run as Firecracker MicroVMs
- Jobs specify compatible worker types
- Scheduler places jobs on correct worker type

---

### Phase 6: Auto-Scaling (3 weeks)
**Goal:** Dynamic worker pool management

**Tasks:**
- [ ] Implement `AutoScaler` with configurable policies
- [ ] Implement `VmProvisioner` trait
- [ ] Implement `FirecrackerProvisioner`
- [ ] Add auto-scaling configuration
- [ ] Add background auto-scaler loop
- [ ] Add metrics (scale-up/down events, queue depth)
- [ ] Test scaling under load

**Deliverables:**
- Workers auto-scale based on queue depth and CPU
- Min/max worker limits respected
- Cooldown prevents thrashing

---

## Timeline Summary

| Phase | Duration | Cumulative | Key Milestone |
|-------|----------|------------|---------------|
| 1. Worker Registry | 4 weeks | 4 weeks | Workers visible and tracked |
| 2. Resource Management | 3 weeks | 7 weeks | Resource-aware scheduling |
| 3. Smart Scheduler | 4 weeks | 11 weeks | Intelligent placement |
| 4. Lifecycle Management | 3 weeks | 14 weeks | Health and restarts |
| 5. Heterogeneous Workers | 4 weeks | 18 weeks | Firecracker support |
| 6. Auto-Scaling | 3 weeks | 21 weeks | Dynamic scaling |

**Total: ~5 months** to full Kubernetes-like orchestrator

---

## Key Decisions

### Pull vs Push Scheduling

**Hybrid Approach Recommended:**

- **Control plane assigns jobs** (push) after running scheduler
- **Workers poll assigned jobs** (pull) from filtered endpoint
- **Best of both worlds:**
  - Scheduler has global view for optimal placement
  - Workers remain resilient (no callbacks needed)
  - Simple worker implementation (just poll different endpoint)

```rust
// Worker loop change:
loop {
    // NEW: Poll for jobs assigned to this worker
    match client.get_assigned_work().await {
        Ok(Some(job)) => { /* execute */ }
        Ok(None) => tokio::time::sleep(Duration::from_secs(2)).await,
        Err(e) => { /* ... */ }
    }
}

// API endpoint:
GET /api/workers/{node_id}/jobs
// Returns jobs with assigned_worker = node_id
```

### Database Schema Evolution

**Use hiqlite migrations:**

```rust
// src/migrations/002_add_workers_table.rs
pub fn migrate(db: &Hiqlite) -> Result<()> {
    db.execute("
        CREATE TABLE IF NOT EXISTS workers (
            node_id TEXT PRIMARY KEY,
            worker_type TEXT NOT NULL,
            status TEXT NOT NULL,
            registered_at INTEGER NOT NULL,
            last_heartbeat INTEGER NOT NULL,
            cpu_millicores INTEGER NOT NULL,
            memory_mb INTEGER NOT NULL,
            disk_mb INTEGER,
            gpu_count INTEGER DEFAULT 0,
            allocated_cpu_millicores INTEGER DEFAULT 0,
            allocated_memory_mb INTEGER DEFAULT 0,
            labels TEXT,
            health_status TEXT,
            health_message TEXT,
            software_version TEXT
        )
    ")?;
    Ok(())
}
```

### Backward Compatibility

**Maintain compatibility during migration:**

- Phase 1: Workers can optionally register (registration not required)
- Phase 2: Resource requirements default to minimal values
- Phase 3: Scheduler coexists with pull-based claiming (gradual migration)
- Old workers (pre-registry) inferred from job activity
- New workers (post-registry) explicitly tracked

---

## Comparison to Kubernetes

| Feature | Kubernetes | mvm-ci Orchestrator |
|---------|-----------|---------------------|
| **Scheduling** | kube-scheduler | BinPackScheduler |
| **Resource Management** | Requests & Limits | ResourceRequirements |
| **Affinity** | Node selectors, affinity/anti-affinity | AffinityRules |
| **Health Checks** | Liveness/readiness probes | Heartbeats + optional HTTP probes |
| **Auto-Scaling** | HPA, Cluster Autoscaler | AutoScaler + VmProvisioner |
| **Networking** | Overlay networks (Calico, Flannel) | iroh+h3 P2P (NAT traversal) |
| **Storage** | etcd (Raft) | hiqlite (Raft SQLite) |
| **Worker Types** | Homogeneous (containers) | Heterogeneous (WASM, VMs, containers) |
| **Complexity** | High (>1M LoC) | Low (~10K LoC) |

**Key Advantage:** P2P networking allows orchestration across NAT boundaries without VPN/VPC setup. Perfect for edge computing, hybrid cloud, and IoT scenarios.

---

## Next Steps

1. **Review design** with team
2. **Prioritize phases** based on immediate needs
3. **Start Phase 1** (Worker Registry) - Foundation for everything else
4. **Create proof-of-concept** Firecracker worker
5. **Write integration tests** for each phase
6. **Document API** using OpenAPI/Swagger

---

## References

- **Kubernetes Scheduler:** https://kubernetes.io/docs/concepts/scheduling-eviction/kube-scheduler/
- **Bin Packing Algorithms:** https://en.wikipedia.org/wiki/Bin_packing_problem
- **Firecracker:** https://firecracker-microvm.github.io/
- **iroh P2P:** https://www.iroh.computer/docs
- **hiqlite:** https://github.com/sebadob/hiqlite
