//! Job worker test utilities for madsim-based distributed testing.
//!
//! This module provides `JobWorkerTester`, a high-level abstraction that extends
//! `AspenRaftTester` with job-specific capabilities for testing cross-node
//! job execution and worker coordination.
//!
//! # Design Principles (Tiger Style)
//!
//! - **Bounded resources**: All operations respect MAX_TEST_WORKERS, MAX_TEST_JOBS limits
//! - **Explicit types**: Uses JobId, NodeId for clear identification
//! - **Fail-fast**: All errors propagate immediately via Result
//! - **Deterministic**: Uses madsim RNG for reproducible tests
//!
//! # Example
//!
//! ```ignore
//! use aspen_testing::job_worker_tester::{JobWorkerTester, JobWorkerTestConfig};
//!
//! #[madsim::test]
//! async fn test_cross_node_job_execution() {
//!     let config = JobWorkerTestConfig::default();
//!     let mut t = JobWorkerTester::new(3, "cross_node_job", config).await;
//!
//!     // Submit a job through the cluster
//!     let job_id = t.submit_job_to_cluster(JobSpec::new("test")).await.unwrap();
//!
//!     // Wait for completion
//!     let job = t.wait_for_job(&job_id, Duration::from_secs(30)).await.unwrap();
//!     assert_eq!(job.status, JobStatus::Completed);
//!
//!     t.end();
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex;

use aspen_coordination::LoadBalancingStrategy;
use aspen_core::inmemory::DeterministicKeyValueStore;
use aspen_jobs::{
    Job, JobId, JobManager, JobResult, JobSpec, JobStatus, Worker, WorkerPool,
};

use aspen_core::simulation::SimulationArtifact;

use crate::madsim_tester::{AspenRaftTester, TesterConfig};

/// Tiger Style: Fixed limits for job worker testing.
const MAX_TEST_WORKERS: usize = 32;
const MAX_TEST_JOBS: usize = 1000;
const MAX_STEAL_BATCH: usize = 10;
const DEFAULT_JOB_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_WORKERS_PER_NODE: usize = 2;

/// Configuration for job worker test scenarios.
#[derive(Debug, Clone)]
pub struct JobWorkerTestConfig {
    /// Base Raft tester configuration.
    pub raft_config: TesterConfig,
    /// Number of workers per node.
    pub workers_per_node: usize,
    /// Load balancing strategy.
    pub strategy: LoadBalancingStrategy,
    /// Enable work stealing.
    pub enable_work_stealing: bool,
    /// Job types to register.
    pub job_types: Vec<String>,
    /// Base execution time for test workers (ms).
    pub base_execution_time_ms: u64,
}

impl Default for JobWorkerTestConfig {
    fn default() -> Self {
        Self {
            raft_config: TesterConfig::new(3, "job_worker_test"),
            workers_per_node: DEFAULT_WORKERS_PER_NODE,
            strategy: LoadBalancingStrategy::LeastLoaded,
            enable_work_stealing: true,
            job_types: vec!["test".to_string()],
            base_execution_time_ms: 100,
        }
    }
}

impl JobWorkerTestConfig {
    /// Create a new config with specified node count and test name.
    pub fn new(node_count: usize, test_name: &str) -> Self {
        Self {
            raft_config: TesterConfig::new(node_count, test_name),
            ..Default::default()
        }
    }

    /// Set load balancing strategy.
    pub fn with_strategy(mut self, strategy: LoadBalancingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set workers per node.
    pub fn with_workers_per_node(mut self, count: usize) -> Self {
        assert!(count <= MAX_TEST_WORKERS, "workers_per_node exceeds MAX_TEST_WORKERS");
        self.workers_per_node = count;
        self
    }

    /// Disable work stealing.
    pub fn without_work_stealing(mut self) -> Self {
        self.enable_work_stealing = false;
        self
    }

    /// Set explicit seed for deterministic testing.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.raft_config.seed = Some(seed);
        self
    }
}

/// Tracked job execution event.
#[derive(Debug, Clone)]
pub struct JobExecutionEvent {
    /// Job ID that was executed.
    pub job_id: JobId,
    /// Worker ID that executed the job.
    pub worker_id: String,
    /// Node index where execution occurred.
    pub node_idx: usize,
    /// Execution start timestamp (relative to test start).
    pub started_at_ms: u64,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Result of execution.
    pub result: JobExecutionResult,
}

/// Result of a tracked job execution.
#[derive(Debug, Clone)]
pub enum JobExecutionResult {
    Success,
    Failure(String),
}

/// Simulated job execution tracker for deterministic testing.
///
/// Tracks all job submissions and executions for verification in tests.
#[derive(Debug)]
pub struct SimulatedJobTracker {
    /// Jobs submitted (node_idx, job_id, timestamp_ms).
    submitted: Mutex<Vec<(usize, JobId, u64)>>,
    /// Job execution events.
    executed: Mutex<Vec<JobExecutionEvent>>,
    /// Test start time for relative timestamps.
    start_time: std::time::Instant,
}

impl Default for SimulatedJobTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedJobTracker {
    /// Create a new tracker.
    pub fn new() -> Self {
        Self {
            submitted: Mutex::new(Vec::new()),
            executed: Mutex::new(Vec::new()),
            start_time: std::time::Instant::now(),
        }
    }

    /// Record a job submission.
    pub async fn record_submission(&self, node_idx: usize, job_id: JobId) {
        let mut submitted = self.submitted.lock().await;
        let timestamp = self.start_time.elapsed().as_millis() as u64;
        submitted.push((node_idx, job_id, timestamp));
    }

    /// Record job execution start.
    pub async fn record_start(&self, job_id: &JobId, worker_id: &str, node_idx: usize) -> u64 {
        let timestamp = self.start_time.elapsed().as_millis() as u64;
        // Store in executed with placeholder result
        let mut executed = self.executed.lock().await;
        executed.push(JobExecutionEvent {
            job_id: job_id.clone(),
            worker_id: worker_id.to_string(),
            node_idx,
            started_at_ms: timestamp,
            duration_ms: 0, // Will be updated on completion
            result: JobExecutionResult::Success, // Default, may be overwritten
        });
        timestamp
    }

    /// Record job execution success.
    pub async fn record_success(&self, job_id: &JobId, duration_ms: u64) {
        let mut executed = self.executed.lock().await;
        if let Some(event) = executed.iter_mut().rev().find(|e| &e.job_id == job_id) {
            event.duration_ms = duration_ms;
            event.result = JobExecutionResult::Success;
        }
    }

    /// Record job execution failure.
    pub async fn record_failure(&self, job_id: &JobId, error: &str, duration_ms: u64) {
        let mut executed = self.executed.lock().await;
        if let Some(event) = executed.iter_mut().rev().find(|e| &e.job_id == job_id) {
            event.duration_ms = duration_ms;
            event.result = JobExecutionResult::Failure(error.to_string());
        }
    }

    /// Get all execution events.
    pub async fn executions(&self) -> Vec<JobExecutionEvent> {
        self.executed.lock().await.clone()
    }

    /// Get jobs executed on a specific node.
    pub async fn jobs_on_node(&self, node_idx: usize) -> Vec<JobId> {
        let executed = self.executed.lock().await;
        executed
            .iter()
            .filter(|e| e.node_idx == node_idx)
            .filter(|e| matches!(e.result, JobExecutionResult::Success))
            .map(|e| e.job_id.clone())
            .collect()
    }

    /// Get count of jobs executed per node.
    pub async fn jobs_per_node(&self) -> HashMap<usize, usize> {
        let executed = self.executed.lock().await;
        let mut counts = HashMap::new();
        for event in executed.iter() {
            if matches!(event.result, JobExecutionResult::Success) {
                *counts.entry(event.node_idx).or_insert(0) += 1;
            }
        }
        counts
    }
}

/// Deterministic test worker that tracks execution for verification.
///
/// This worker simulates job execution with configurable:
/// - Execution time (deterministic via madsim RNG)
/// - Failure rate for fault injection
/// - Failure conditions for specific scenarios
pub struct DeterministicTestWorker {
    /// Node index this worker belongs to.
    node_idx: usize,
    /// Unique worker identifier.
    worker_id: String,
    /// Tracker for recording executions.
    tracker: Arc<SimulatedJobTracker>,
    /// Base execution time in milliseconds.
    base_execution_time_ms: u64,
    /// Probability of random failure (0.0 to 1.0).
    failure_rate: f64,
    /// Supported job types (empty = all).
    supported_job_types: Vec<String>,
}

impl DeterministicTestWorker {
    /// Create a new test worker.
    pub fn new(
        node_idx: usize,
        worker_id: String,
        tracker: Arc<SimulatedJobTracker>,
        base_execution_time_ms: u64,
    ) -> Self {
        Self {
            node_idx,
            worker_id,
            tracker,
            base_execution_time_ms,
            failure_rate: 0.0,
            supported_job_types: vec!["test".to_string()],
        }
    }

    /// Set failure rate (0.0 to 1.0).
    pub fn with_failure_rate(mut self, rate: f64) -> Self {
        self.failure_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Set supported job types.
    pub fn with_job_types(mut self, types: Vec<String>) -> Self {
        self.supported_job_types = types;
        self
    }

    fn should_fail(&self) -> bool {
        if self.failure_rate <= 0.0 {
            return false;
        }
        let random: f64 = (madsim::rand::random::<u64>() as f64) / (u64::MAX as f64);
        random < self.failure_rate
    }
}

#[async_trait]
impl Worker for DeterministicTestWorker {
    async fn execute(&self, job: Job) -> JobResult {
        // Check for force_fail in payload
        let force_fail = job
            .spec
            .payload
            .get("force_fail")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Record execution start
        let start_time = self
            .tracker
            .record_start(&job.id, &self.worker_id, self.node_idx)
            .await;

        // Simulate work (deterministic via madsim)
        let jitter = madsim::rand::random::<u64>() % 50;
        let execution_time = self.base_execution_time_ms + jitter;
        madsim::time::sleep(Duration::from_millis(execution_time)).await;

        let duration = (std::time::Instant::now().elapsed().as_millis() as u64)
            .saturating_sub(start_time);

        // Check failure conditions
        if force_fail || self.should_fail() {
            let error = "simulated failure";
            self.tracker.record_failure(&job.id, error, duration).await;
            return JobResult::failure(error);
        }

        // Record success
        self.tracker.record_success(&job.id, duration).await;
        JobResult::success(json!({
            "executed_on_node": self.node_idx,
            "worker_id": self.worker_id,
            "duration_ms": execution_time
        }))
    }

    fn job_types(&self) -> Vec<String> {
        self.supported_job_types.clone()
    }
}

/// Worker load statistics for a node.
#[derive(Debug, Clone, Default)]
pub struct WorkerLoadStats {
    /// Number of workers on this node.
    pub worker_count: usize,
    /// Number of idle workers.
    pub idle_workers: usize,
    /// Number of workers currently processing.
    pub processing_workers: usize,
    /// Current load (0.0 to 1.0).
    pub load: f32,
    /// Queue depth.
    pub queue_depth: usize,
}

/// Work stealing result.
#[derive(Debug, Clone)]
pub struct WorkStealingResult {
    /// Number of jobs stolen.
    pub jobs_stolen: usize,
    /// Source node.
    pub from_node: usize,
    /// Target node.
    pub to_node: usize,
}

/// High-level tester for job worker integration tests.
///
/// Wraps `AspenRaftTester` and adds job-specific testing capabilities
/// including job submission, worker tracking, and load distribution analysis.
pub struct JobWorkerTester {
    /// Underlying Raft tester.
    raft: AspenRaftTester,
    /// Job managers per node (indexed by node idx).
    /// Note: JobManager<S> takes Arc<S>, so S = DeterministicKeyValueStore.
    job_managers: Vec<Arc<JobManager<DeterministicKeyValueStore>>>,
    /// Worker pools per node.
    worker_pools: Vec<Arc<WorkerPool<DeterministicKeyValueStore>>>,
    /// Shared KV stores per node.
    stores: Vec<Arc<DeterministicKeyValueStore>>,
    /// Job execution tracker.
    tracker: Arc<SimulatedJobTracker>,
    /// Submitted job IDs for tracking.
    submitted_jobs: Vec<JobId>,
    /// Test configuration.
    config: JobWorkerTestConfig,
    /// Simulated worker loads per node.
    simulated_loads: HashMap<usize, f32>,
    /// Whether workers are enabled per node.
    workers_enabled: HashMap<usize, bool>,
}

impl JobWorkerTester {
    /// Create a new job worker tester with n nodes.
    pub async fn new(n: usize, test_name: &str, config: JobWorkerTestConfig) -> Self {
        assert!(n > 0 && n <= 64, "node_count must be between 1 and 64");
        assert!(
            config.workers_per_node * n <= MAX_TEST_WORKERS,
            "total workers exceed MAX_TEST_WORKERS"
        );

        // Create Raft tester with updated config
        let mut raft_config = config.raft_config.clone();
        raft_config.node_count = n;
        raft_config.test_name = test_name.to_string();
        let raft = AspenRaftTester::with_config(raft_config).await;

        // Create shared infrastructure
        let tracker = Arc::new(SimulatedJobTracker::new());
        let mut job_managers: Vec<Arc<JobManager<DeterministicKeyValueStore>>> = Vec::with_capacity(n);
        let mut worker_pools: Vec<Arc<WorkerPool<DeterministicKeyValueStore>>> = Vec::with_capacity(n);
        let mut stores: Vec<Arc<DeterministicKeyValueStore>> = Vec::with_capacity(n);
        let mut workers_enabled = HashMap::new();

        // Initialize job manager and worker pool for each node
        for i in 0..n {
            // Create the KV store for this node
            // Note: DeterministicKeyValueStore::new() already returns Arc<Self>
            let store = DeterministicKeyValueStore::new();

            // Store a reference for tracking, then use store for the manager
            stores.push(store.clone());

            // Create job manager - JobManager<S>::new(Arc<S>) where S = DeterministicKeyValueStore
            let manager = Arc::new(JobManager::new(store));

            // Initialize the job manager
            manager.initialize().await.expect("failed to initialize job manager");

            let pool = Arc::new(WorkerPool::with_manager(manager.clone()));

            // Register test workers
            for w in 0..config.workers_per_node {
                let worker_id = format!("node-{}-worker-{}", i, w);
                let worker = DeterministicTestWorker::new(
                    i,
                    worker_id,
                    tracker.clone(),
                    config.base_execution_time_ms,
                )
                .with_job_types(config.job_types.clone());

                pool.register_handler("test", worker)
                    .await
                    .expect("failed to register worker");
            }

            // Start workers
            pool.start(config.workers_per_node)
                .await
                .expect("failed to start workers");

            job_managers.push(manager);
            worker_pools.push(pool);
            workers_enabled.insert(i, true);
        }

        Self {
            raft,
            job_managers,
            worker_pools,
            stores,
            tracker,
            submitted_jobs: Vec::new(),
            config,
            simulated_loads: HashMap::new(),
            workers_enabled,
        }
    }

    /// Get the underlying Raft tester for advanced operations.
    pub fn raft(&mut self) -> &mut AspenRaftTester {
        &mut self.raft
    }

    /// Get the job execution tracker.
    pub fn tracker(&self) -> &SimulatedJobTracker {
        &self.tracker
    }

    /// Submit a job to a specific node.
    pub async fn submit_job(&mut self, node_idx: usize, spec: JobSpec) -> Result<JobId> {
        assert!(node_idx < self.job_managers.len(), "invalid node index");
        assert!(
            self.submitted_jobs.len() < MAX_TEST_JOBS,
            "exceeded MAX_TEST_JOBS"
        );

        let manager = &self.job_managers[node_idx];
        let job_id = manager.submit(spec).await?;

        self.tracker.record_submission(node_idx, job_id.clone()).await;
        self.submitted_jobs.push(job_id.clone());

        Ok(job_id)
    }

    /// Submit a job to the cluster (routes to appropriate node based on strategy).
    ///
    /// For simplicity in tests, this submits to node 0 (which is typically the leader).
    pub async fn submit_job_to_cluster(&mut self, spec: JobSpec) -> Result<JobId> {
        // For load-based routing, find the least loaded node
        let target_node = match self.config.strategy {
            LoadBalancingStrategy::LeastLoaded => {
                let mut best_node = 0;
                let mut best_load = f32::MAX;
                for (node_idx, &enabled) in &self.workers_enabled {
                    if enabled {
                        let load = self.simulated_loads.get(node_idx).copied().unwrap_or(0.0);
                        if load < best_load {
                            best_load = load;
                            best_node = *node_idx;
                        }
                    }
                }
                best_node
            }
            LoadBalancingStrategy::RoundRobin => {
                // Simple round-robin based on number of submitted jobs
                self.submitted_jobs.len() % self.job_managers.len()
            }
            _ => 0, // Default to node 0 for other strategies
        };

        self.submit_job(target_node, spec).await
    }

    /// Wait for a specific job to complete.
    pub async fn wait_for_job(&self, job_id: &JobId, timeout: Duration) -> Result<Job> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(50);

        while start.elapsed() < timeout {
            // Check all managers for the job (it could be on any node)
            for manager in &self.job_managers {
                if let Ok(Some(job)) = manager.get_job(job_id).await {
                    match job.status {
                        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled => {
                            return Ok(job);
                        }
                        _ => {}
                    }
                }
            }
            madsim::time::sleep(poll_interval).await;
        }

        anyhow::bail!("timeout waiting for job {} to complete", job_id)
    }

    /// Wait for all submitted jobs to complete.
    pub async fn wait_for_all_jobs(&self, timeout: Duration) -> Result<Vec<Job>> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(100);
        let job_ids = self.submitted_jobs.clone();

        while start.elapsed() < timeout {
            let mut all_complete = true;
            let mut completed_jobs = Vec::new();

            for job_id in &job_ids {
                let mut found = false;
                for manager in &self.job_managers {
                    if let Ok(Some(job)) = manager.get_job(job_id).await {
                        match job.status {
                            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled => {
                                completed_jobs.push(job);
                                found = true;
                                break;
                            }
                            _ => {
                                all_complete = false;
                                found = true;
                                break;
                            }
                        }
                    }
                }
                if !found {
                    all_complete = false;
                }
            }

            if all_complete && completed_jobs.len() == job_ids.len() {
                return Ok(completed_jobs);
            }

            madsim::time::sleep(poll_interval).await;
        }

        anyhow::bail!("timeout waiting for all {} jobs to complete", job_ids.len())
    }

    /// Get a job from any node.
    pub async fn get_job(&self, job_id: &JobId) -> Result<Job> {
        for manager in &self.job_managers {
            if let Ok(Some(job)) = manager.get_job(job_id).await {
                return Ok(job);
            }
        }
        anyhow::bail!("job {} not found", job_id)
    }

    /// Get the node where a job was executed.
    pub async fn get_execution_node(&self, job_id: &JobId) -> Option<usize> {
        let executions = self.tracker.executions().await;
        executions
            .iter()
            .find(|e| &e.job_id == job_id)
            .map(|e| e.node_idx)
    }

    /// Get count of jobs executed per node.
    pub async fn jobs_executed_per_node(&self) -> HashMap<usize, usize> {
        self.tracker.jobs_per_node().await
    }

    /// Set simulated load for a node (affects routing decisions).
    pub fn set_node_worker_load(&mut self, node_idx: usize, load: f32) {
        assert!(node_idx < self.job_managers.len(), "invalid node index");
        self.simulated_loads.insert(node_idx, load.clamp(0.0, 1.0));
    }

    /// Disable workers on a node (jobs will queue but not execute).
    pub async fn disable_workers_on_node(&mut self, node_idx: usize) {
        assert!(node_idx < self.worker_pools.len(), "invalid node index");
        self.workers_enabled.insert(node_idx, false);
        // Note: In a full implementation, we'd actually stop the workers.
        // For now, we just track the state.
    }

    /// Enable workers on a node.
    pub async fn enable_workers_on_node(&mut self, node_idx: usize) {
        assert!(node_idx < self.worker_pools.len(), "invalid node index");
        self.workers_enabled.insert(node_idx, true);
    }

    /// Get queue depth on a node.
    pub async fn get_queue_depth(&self, _node_idx: usize) -> usize {
        // In a full implementation, this would query the job manager's queue
        // For now, return a placeholder
        0
    }

    /// Crash a node (delegates to Raft tester).
    pub async fn crash_node(&mut self, node_idx: usize) {
        self.raft.crash_node(node_idx).await;
        self.workers_enabled.insert(node_idx, false);
    }

    /// Restart a node (delegates to Raft tester).
    pub async fn restart_node(&mut self, node_idx: usize) {
        self.raft.restart_node(node_idx).await;
        self.workers_enabled.insert(node_idx, true);
    }

    /// Disconnect a node from the network.
    pub fn disconnect(&mut self, node_idx: usize) {
        self.raft.disconnect(node_idx);
    }

    /// Reconnect a node to the network.
    pub fn connect(&mut self, node_idx: usize) {
        self.raft.connect(node_idx);
    }

    /// Add an event to the simulation artifact.
    pub fn add_event(&mut self, event: String) {
        // The raft tester handles artifact management internally
        // For now, just log to stderr for debugging
        eprintln!("[JobWorkerTester] {}", event);
    }

    /// Check for exactly one leader in the Raft cluster.
    pub async fn check_one_leader(&mut self) -> Option<usize> {
        self.raft.check_one_leader().await
    }

    /// End the test and return simulation artifact.
    pub fn end(self) -> SimulationArtifact {
        // Shutdown worker pools
        // Note: In madsim, we don't need to explicitly shutdown as the test ends
        self.raft.end()
    }
}
