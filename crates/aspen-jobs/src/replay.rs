//! Deterministic job replay with madsim for testing and debugging.
//!
//! This module provides deterministic simulation of job execution,
//! allowing replay of job sequences with controlled timing and failures.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_constants::network::MAX_JOB_SPEC_SIZE;
use aspen_hlc::HLC;
use aspen_hlc::SerializableTimestamp;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::Result;
use crate::job::Job;
use crate::job::JobId;
use crate::job::JobResult;
use crate::job::JobSpec;
use crate::manager::JobManager;

/// Deterministic job replay system.
pub struct JobReplaySystem {
    /// Recorded job events.
    events: Vec<JobEvent>,
    /// Current replay position (bounded by event count).
    position: u32,
    /// Simulation seed for deterministic randomness.
    seed: u64,
    /// Replay configuration.
    config: ReplayConfig,
    /// HLC for timestamping events.
    hlc: HLC,
}

impl JobReplaySystem {
    /// Create a new replay system.
    pub fn new(seed: u64, node_id: &str) -> Self {
        Self {
            events: Vec::new(),
            position: 0,
            seed,
            config: ReplayConfig::default(),
            hlc: aspen_core::hlc::create_hlc(node_id),
        }
    }

    /// Record a job submission.
    pub fn record_submission(&mut self, job: &JobSpec) {
        self.events.push(JobEvent::Submitted {
            job: job.clone(),
            hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
        });
    }

    /// Record job execution start.
    pub fn record_execution(&mut self, job_id: JobId, worker_id: String) {
        self.events.push(JobEvent::Started {
            job_id,
            worker_id,
            hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
        });
    }

    /// Record job completion.
    pub fn record_completion(&mut self, job_id: JobId, result: JobResult, duration_ms: u64) {
        self.events.push(JobEvent::Completed {
            job_id,
            result,
            duration_ms,
            hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
        });
    }

    /// Record a failure.
    pub fn record_failure(&mut self, job_id: JobId, error: String) {
        self.events.push(JobEvent::Failed {
            job_id,
            error,
            hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
        });
    }

    /// Record network partition.
    pub fn record_partition(&mut self, nodes: Vec<String>) {
        self.events.push(JobEvent::NetworkPartition {
            nodes,
            hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
        });
    }

    /// Record crash.
    pub fn record_crash(&mut self, node_id: String) {
        self.events.push(JobEvent::NodeCrash {
            node_id,
            hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
        });
    }

    /// Save replay to file.
    pub fn save(&self, path: &str) -> Result<()> {
        let data = ReplayData {
            seed: self.seed,
            events: self.events.clone(),
            config: self.config.clone(),
        };

        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| crate::error::JobError::SerializationError { source: e })?;

        std::fs::write(path, json).map_err(|_e| crate::error::JobError::InvalidJobSpec {
            reason: format!("Failed to write replay file to {}", path),
        })?;

        info!("saved replay to {}", path);
        Ok(())
    }

    /// Load replay from file.
    ///
    /// Creates a new HLC instance for the loaded replay using the provided node_id.
    ///
    /// Tiger Style: Validates file size before reading to prevent memory exhaustion.
    pub fn load(path: &str, node_id: &str) -> Result<Self> {
        // Tiger Style: Check file size before reading
        let metadata = std::fs::metadata(path).map_err(|_e| crate::error::JobError::InvalidJobSpec {
            reason: format!("Failed to stat replay file at {}", path),
        })?;
        if metadata.len() > MAX_JOB_SPEC_SIZE {
            return Err(crate::error::JobError::InvalidJobSpec {
                reason: format!("Replay file too large: {} bytes (max {})", metadata.len(), MAX_JOB_SPEC_SIZE),
            });
        }

        let data = std::fs::read_to_string(path).map_err(|_e| crate::error::JobError::InvalidJobSpec {
            reason: format!("Failed to read replay file from {}", path),
        })?;

        let replay_data: ReplayData =
            serde_json::from_str(&data).map_err(|e| crate::error::JobError::SerializationError { source: e })?;

        Ok(Self {
            events: replay_data.events,
            position: 0,
            seed: replay_data.seed,
            config: replay_data.config,
            hlc: aspen_core::hlc::create_hlc(node_id),
        })
    }

    /// Get next event in replay.
    pub fn next_event(&mut self) -> Option<&JobEvent> {
        if (self.position as usize) < self.events.len() {
            let event = &self.events[self.position as usize];
            self.position += 1;
            Some(event)
        } else {
            None
        }
    }

    /// Reset replay position.
    pub fn reset(&mut self) {
        self.position = 0;
    }

    /// Get all events.
    pub fn events(&self) -> &[JobEvent] {
        &self.events
    }

    /// Set replay configuration.
    pub fn with_config(mut self, config: ReplayConfig) -> Self {
        self.config = config;
        self
    }
}

/// Job event for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs, clippy::large_enum_variant)]
pub enum JobEvent {
    /// Job submitted.
    Submitted {
        /// The job specification.
        job: JobSpec,
        /// HLC timestamp when the job was submitted.
        hlc_timestamp: SerializableTimestamp,
    },
    /// Job started execution.
    Started {
        /// The job ID.
        job_id: JobId,
        /// The worker handling the job.
        worker_id: String,
        /// HLC timestamp when execution started.
        hlc_timestamp: SerializableTimestamp,
    },
    /// Job completed.
    Completed {
        /// The job ID.
        job_id: JobId,
        /// The execution result.
        result: JobResult,
        /// Execution duration in milliseconds.
        duration_ms: u64,
        /// HLC timestamp when the job completed.
        hlc_timestamp: SerializableTimestamp,
    },
    /// Job failed.
    Failed {
        /// The job ID.
        job_id: JobId,
        /// Error message.
        error: String,
        /// HLC timestamp when the job failed.
        hlc_timestamp: SerializableTimestamp,
    },
    /// Network partition occurred.
    NetworkPartition {
        /// Nodes affected by the partition.
        nodes: Vec<String>,
        /// HLC timestamp when the partition occurred.
        hlc_timestamp: SerializableTimestamp,
    },
    /// Node crashed.
    NodeCrash {
        /// The crashed node ID.
        node_id: String,
        /// HLC timestamp when the crash occurred.
        hlc_timestamp: SerializableTimestamp,
    },
    /// Worker pool scaled.
    WorkerScaled {
        /// The pool ID.
        pool_id: String,
        /// New pool size (bounded by worker limits).
        new_size: u32,
        /// HLC timestamp when the scaling occurred.
        hlc_timestamp: SerializableTimestamp,
    },
    /// Custom event.
    Custom {
        /// Event name.
        name: String,
        /// Event data.
        data: serde_json::Value,
        /// HLC timestamp when the event occurred.
        hlc_timestamp: SerializableTimestamp,
    },
}

/// Replay configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    /// Speed multiplier for replay.
    pub speed: f64,
    /// Enable chaos injection.
    pub chaos_enabled: bool,
    /// Failure injection rate (0.0-1.0).
    pub failure_rate: f32,
    /// Network delay range (ms).
    pub network_delay: (u64, u64),
    /// Enable deterministic mode.
    pub deterministic: bool,
    /// Maximum replay duration.
    pub max_duration_ms: u64,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            speed: 1.0,
            chaos_enabled: false,
            failure_rate: 0.0,
            network_delay: (0, 0),
            deterministic: true,
            max_duration_ms: 60_000,
        }
    }
}

/// Replay data for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReplayData {
    seed: u64,
    events: Vec<JobEvent>,
    config: ReplayConfig,
}

/// Deterministic job executor for simulations.
pub struct DeterministicJobExecutor {
    /// Job execution history.
    history: HashMap<JobId, ExecutionRecord>,
    /// Random seed.
    seed: u64,
    /// Failure injection.
    inject_failures: bool,
    /// Network delays.
    network_delays: HashMap<String, Duration>,
    /// HLC for timestamping.
    hlc: HLC,
}

impl DeterministicJobExecutor {
    /// Create a new deterministic executor.
    pub fn new(seed: u64, node_id: &str) -> Self {
        Self {
            history: HashMap::new(),
            seed,
            inject_failures: false,
            network_delays: HashMap::new(),
            hlc: aspen_core::hlc::create_hlc(node_id),
        }
    }

    /// Enable failure injection.
    pub fn with_failures(mut self, rate: f32) -> Self {
        self.inject_failures = rate > 0.0;
        self
    }

    /// Add network delay for a node.
    pub fn add_network_delay(&mut self, node: String, delay: Duration) {
        self.network_delays.insert(node, delay);
    }

    /// Execute a job deterministically.
    pub async fn execute(&mut self, job: &Job) -> JobResult {
        // Record execution start
        let record = ExecutionRecord {
            job_id: job.id.clone(),
            start_hlc: self.current_hlc_timestamp(),
            end_hlc: None,
            result: None,
            retries: 0,
        };

        self.history.insert(job.id.clone(), record);

        // Simulate network delay
        if let Some(delay) = self.get_network_delay(&job.id) {
            tokio::time::sleep(delay).await;
        }

        // Deterministic execution based on seed
        let should_fail = self.should_fail(&job.id);

        let result = if should_fail {
            JobResult::failure("Injected failure for testing".to_string())
        } else {
            // Simulate successful execution
            let duration = self.get_execution_duration(&job.id);
            tokio::time::sleep(Duration::from_millis(duration)).await;

            JobResult::success(serde_json::json!({
                "executed_at_ms": self.current_hlc_timestamp().to_unix_ms(),
                "seed": self.seed,
                "deterministic": true,
            }))
        };

        // Update history
        let end_hlc = self.current_hlc_timestamp();
        if let Some(record) = self.history.get_mut(&job.id) {
            record.end_hlc = Some(end_hlc);
            record.result = Some(result.clone());
        }

        result
    }

    /// Check if job should fail based on deterministic seed.
    fn should_fail(&self, job_id: &JobId) -> bool {
        if !self.inject_failures {
            return false;
        }

        // Use job ID and seed to deterministically decide failure
        let hash = self.hash_job_id(job_id);
        (hash % 100) < 10 // 10% failure rate
    }

    /// Get deterministic execution duration.
    fn get_execution_duration(&self, job_id: &JobId) -> u64 {
        let hash = self.hash_job_id(job_id);
        100 + (hash % 400) // 100-500ms
    }

    /// Get network delay for job.
    fn get_network_delay(&self, job_id: &JobId) -> Option<Duration> {
        // Simplified: use job ID to determine if delay should apply
        if self.network_delays.is_empty() {
            return None;
        }

        let hash = self.hash_job_id(job_id);
        if hash.is_multiple_of(3) {
            // Apply delay to ~33% of jobs
            Some(Duration::from_millis(50 + (hash % 150)))
        } else {
            None
        }
    }

    /// Hash job ID for deterministic behavior.
    fn hash_job_id(&self, job_id: &JobId) -> u64 {
        // Simple hash combining seed and job ID
        let id_str = job_id.to_string();
        let mut hash = self.seed;
        for byte in id_str.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash
    }

    /// Get current HLC timestamp.
    fn current_hlc_timestamp(&self) -> SerializableTimestamp {
        SerializableTimestamp::from(self.hlc.new_timestamp())
    }

    /// Get execution history.
    pub fn history(&self) -> &HashMap<JobId, ExecutionRecord> {
        &self.history
    }
}

/// Execution record for a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Job ID.
    pub job_id: JobId,
    /// Start HLC timestamp.
    pub start_hlc: SerializableTimestamp,
    /// End HLC timestamp.
    pub end_hlc: Option<SerializableTimestamp>,
    /// Execution result.
    pub result: Option<JobResult>,
    /// Number of retries.
    pub retries: u32,
}

/// Replay runner for executing recorded job sequences.
pub struct ReplayRunner<S: aspen_core::KeyValueStore + ?Sized> {
    replay_system: JobReplaySystem,
    manager: Arc<JobManager<S>>,
    #[allow(dead_code)] // Reserved for future deterministic execution integration
    executor: DeterministicJobExecutor,
    stats: ReplayStats,
}

impl<S: aspen_core::KeyValueStore + ?Sized + 'static> ReplayRunner<S> {
    /// Create a new replay runner.
    pub fn new(replay_system: JobReplaySystem, manager: Arc<JobManager<S>>, node_id: &str) -> Self {
        let seed = replay_system.seed;
        Self {
            replay_system,
            manager,
            executor: DeterministicJobExecutor::new(seed, node_id),
            stats: ReplayStats::default(),
        }
    }

    /// Run the replay.
    pub async fn run(&mut self) -> Result<ReplayStats> {
        info!("starting replay with seed {}", self.replay_system.seed);

        self.replay_system.reset();
        let start_time = std::time::Instant::now();

        loop {
            let event = self.replay_system.next_event().cloned();
            if let Some(event) = event {
                self.process_event(event).await?;
            } else {
                break;
            }

            // Check timeout
            if start_time.elapsed().as_millis() > self.replay_system.config.max_duration_ms as u128 {
                warn!("replay timeout reached");
                break;
            }
        }

        self.stats.duration_ms = start_time.elapsed().as_millis() as u64;
        info!("replay completed: {:?}", self.stats);

        Ok(self.stats.clone())
    }

    /// Process a single replay event.
    async fn process_event(&mut self, event: JobEvent) -> Result<()> {
        match event {
            JobEvent::Submitted { job, hlc_timestamp } => {
                debug!("replaying job submission at {}", hlc_timestamp.to_unix_ms());
                let job_id = self.manager.submit(job).await?;
                self.stats.jobs_submitted += 1;
                debug!("submitted job {}", job_id);
            }
            JobEvent::Started {
                job_id,
                worker_id,
                hlc_timestamp,
            } => {
                debug!("replaying job start: {} on {} at {}", job_id, worker_id, hlc_timestamp.to_unix_ms());
                self.stats.jobs_started += 1;
            }
            JobEvent::Completed {
                job_id,
                result: _,
                duration_ms,
                hlc_timestamp,
            } => {
                debug!("replaying job completion: {} in {}ms at {}", job_id, duration_ms, hlc_timestamp.to_unix_ms());
                self.stats.jobs_completed += 1;
                self.stats.total_execution_time_ms += duration_ms;
            }
            JobEvent::Failed {
                job_id,
                error,
                hlc_timestamp,
            } => {
                debug!("replaying job failure: {} - {} at {}", job_id, error, hlc_timestamp.to_unix_ms());
                self.stats.jobs_failed += 1;
            }
            JobEvent::NetworkPartition { nodes, hlc_timestamp } => {
                warn!("replaying network partition at {}: {:?}", hlc_timestamp.to_unix_ms(), nodes);
                self.stats.network_partitions += 1;
            }
            JobEvent::NodeCrash { node_id, hlc_timestamp } => {
                warn!("replaying node crash: {} at {}", node_id, hlc_timestamp.to_unix_ms());
                self.stats.node_crashes += 1;
            }
            _ => {
                debug!("replaying event: {:?}", event);
            }
        }

        // Apply configured delays
        if self.replay_system.config.speed != 1.0 {
            let delay = (10.0 / self.replay_system.config.speed) as u64;
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        Ok(())
    }

    /// Get replay statistics.
    pub fn stats(&self) -> &ReplayStats {
        &self.stats
    }
}

/// Statistics from replay execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplayStats {
    /// Number of jobs submitted.
    pub jobs_submitted: u32,
    /// Number of jobs started.
    pub jobs_started: u32,
    /// Number of jobs completed.
    pub jobs_completed: u32,
    /// Number of jobs failed.
    pub jobs_failed: u32,
    /// Total execution time.
    pub total_execution_time_ms: u64,
    /// Number of network partitions.
    pub network_partitions: u32,
    /// Number of node crashes.
    pub node_crashes: u32,
    /// Replay duration.
    pub duration_ms: u64,
}

#[cfg(madsim)]
mod madsim_integration {
    use madsim::runtime::Runtime;

    use super::*;

    /// Run job replay in madsim simulation.
    pub async fn run_in_simulation(replay_path: &str, seed: u64, node_id: &str) -> Result<ReplayStats> {
        let runtime = Runtime::new();

        let node_id_owned = node_id.to_string();
        runtime.block_on(async {
            // Load replay
            let replay = JobReplaySystem::load(replay_path, &node_id_owned)?;

            // Create simulated environment
            let store = Arc::new(aspen_testing::DeterministicKeyValueStore::new());
            let manager = Arc::new(JobManager::new(store));
            manager.initialize().await?;

            // Run replay
            let mut runner = ReplayRunner::new(replay, manager, &node_id_owned);
            runner.run().await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_save_load() {
        let mut replay = JobReplaySystem::new(12345, "test-node");

        // Record some events
        let job = JobSpec::new("test_job").payload(serde_json::json!({"test": true})).unwrap();
        replay.record_submission(&job);

        let job_id = JobId::new();
        replay.record_execution(job_id.clone(), "worker-1".to_string());
        replay.record_completion(job_id, JobResult::success(serde_json::json!({})), 200);

        // Save and load
        let path = "/tmp/test_replay.json";
        replay.save(path).unwrap();

        let loaded = JobReplaySystem::load(path, "test-node").unwrap();
        assert_eq!(loaded.seed, 12345);
        assert_eq!(loaded.events.len(), 3);
    }

    #[tokio::test]
    async fn test_deterministic_executor() {
        let mut executor = DeterministicJobExecutor::new(42, "test-node");

        let job = Job::from_spec(JobSpec::new("test").payload(serde_json::json!({})).unwrap());

        let result = executor.execute(&job).await;
        assert!(result.is_success());

        // Verify deterministic behavior
        let duration1 = executor.get_execution_duration(&job.id);
        let duration2 = executor.get_execution_duration(&job.id);
        assert_eq!(duration1, duration2);
    }
}
