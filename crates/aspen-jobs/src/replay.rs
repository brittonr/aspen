//! Deterministic job replay with madsim for testing and debugging.
//!
//! This module provides deterministic simulation of job execution,
//! allowing replay of job sequences with controlled timing and failures.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::Result;
use crate::job::{Job, JobId, JobResult, JobSpec, JobStatus};
use crate::manager::JobManager;

/// Deterministic job replay system.
pub struct JobReplaySystem {
    /// Recorded job events.
    events: Vec<JobEvent>,
    /// Current replay position.
    position: usize,
    /// Simulation seed for deterministic randomness.
    seed: u64,
    /// Replay configuration.
    config: ReplayConfig,
}

impl JobReplaySystem {
    /// Create a new replay system.
    pub fn new(seed: u64) -> Self {
        Self {
            events: Vec::new(),
            position: 0,
            seed,
            config: ReplayConfig::default(),
        }
    }

    /// Record a job submission.
    pub fn record_submission(&mut self, job: &JobSpec, timestamp: u64) {
        self.events.push(JobEvent::Submitted {
            job: job.clone(),
            timestamp,
        });
    }

    /// Record job execution start.
    pub fn record_execution(&mut self, job_id: JobId, worker_id: String, timestamp: u64) {
        self.events.push(JobEvent::Started {
            job_id,
            worker_id,
            timestamp,
        });
    }

    /// Record job completion.
    pub fn record_completion(&mut self, job_id: JobId, result: JobResult, duration_ms: u64, timestamp: u64) {
        self.events.push(JobEvent::Completed {
            job_id,
            result,
            duration_ms,
            timestamp,
        });
    }

    /// Record a failure.
    pub fn record_failure(&mut self, job_id: JobId, error: String, timestamp: u64) {
        self.events.push(JobEvent::Failed {
            job_id,
            error,
            timestamp,
        });
    }

    /// Record network partition.
    pub fn record_partition(&mut self, nodes: Vec<String>, timestamp: u64) {
        self.events.push(JobEvent::NetworkPartition { nodes, timestamp });
    }

    /// Record crash.
    pub fn record_crash(&mut self, node_id: String, timestamp: u64) {
        self.events.push(JobEvent::NodeCrash { node_id, timestamp });
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
    pub fn load(path: &str) -> Result<Self> {
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
        })
    }

    /// Get next event in replay.
    pub fn next_event(&mut self) -> Option<&JobEvent> {
        if self.position < self.events.len() {
            let event = &self.events[self.position];
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
pub enum JobEvent {
    /// Job submitted.
    Submitted { job: JobSpec, timestamp: u64 },
    /// Job started execution.
    Started {
        job_id: JobId,
        worker_id: String,
        timestamp: u64,
    },
    /// Job completed.
    Completed {
        job_id: JobId,
        result: JobResult,
        duration_ms: u64,
        timestamp: u64,
    },
    /// Job failed.
    Failed {
        job_id: JobId,
        error: String,
        timestamp: u64,
    },
    /// Network partition occurred.
    NetworkPartition { nodes: Vec<String>, timestamp: u64 },
    /// Node crashed.
    NodeCrash { node_id: String, timestamp: u64 },
    /// Worker pool scaled.
    WorkerScaled {
        pool_id: String,
        new_size: usize,
        timestamp: u64,
    },
    /// Custom event.
    Custom {
        name: String,
        data: serde_json::Value,
        timestamp: u64,
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
}

impl DeterministicJobExecutor {
    /// Create a new deterministic executor.
    pub fn new(seed: u64) -> Self {
        Self {
            history: HashMap::new(),
            seed,
            inject_failures: false,
            network_delays: HashMap::new(),
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
            start_time: self.current_time(),
            end_time: 0,
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
                "executed_at": self.current_time(),
                "seed": self.seed,
                "deterministic": true,
            }))
        };

        // Update history
        let end_time = self.current_time();
        if let Some(record) = self.history.get_mut(&job.id) {
            record.end_time = end_time;
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
        if hash % 3 == 0 {
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

    /// Get current simulation time.
    fn current_time(&self) -> u64 {
        // In real madsim, this would use simulation time
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64
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
    /// Start timestamp.
    pub start_time: u64,
    /// End timestamp.
    pub end_time: u64,
    /// Execution result.
    pub result: Option<JobResult>,
    /// Number of retries.
    pub retries: u32,
}

/// Replay runner for executing recorded job sequences.
pub struct ReplayRunner<S: aspen_core::KeyValueStore + ?Sized> {
    replay_system: JobReplaySystem,
    manager: Arc<JobManager<S>>,
    executor: DeterministicJobExecutor,
    stats: ReplayStats,
}

impl<S: aspen_core::KeyValueStore + ?Sized + 'static> ReplayRunner<S> {
    /// Create a new replay runner.
    pub fn new(replay_system: JobReplaySystem, manager: Arc<JobManager<S>>) -> Self {
        let seed = replay_system.seed;
        Self {
            replay_system,
            manager,
            executor: DeterministicJobExecutor::new(seed),
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
            JobEvent::Submitted { job, timestamp } => {
                debug!("replaying job submission at {}", timestamp);
                let job_id = self.manager.submit(job).await?;
                self.stats.jobs_submitted += 1;
                debug!("submitted job {}", job_id);
            }
            JobEvent::Started {
                job_id,
                worker_id,
                timestamp,
            } => {
                debug!("replaying job start: {} on {} at {}", job_id, worker_id, timestamp);
                self.stats.jobs_started += 1;
            }
            JobEvent::Completed {
                job_id,
                result: _,
                duration_ms,
                timestamp,
            } => {
                debug!("replaying job completion: {} in {}ms at {}", job_id, duration_ms, timestamp);
                self.stats.jobs_completed += 1;
                self.stats.total_execution_time_ms += duration_ms;
            }
            JobEvent::Failed {
                job_id,
                error,
                timestamp,
            } => {
                debug!("replaying job failure: {} - {} at {}", job_id, error, timestamp);
                self.stats.jobs_failed += 1;
            }
            JobEvent::NetworkPartition { nodes, timestamp } => {
                warn!("replaying network partition at {}: {:?}", timestamp, nodes);
                self.stats.network_partitions += 1;
            }
            JobEvent::NodeCrash { node_id, timestamp } => {
                warn!("replaying node crash: {} at {}", node_id, timestamp);
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

#[cfg(feature = "madsim")]
mod madsim_integration {
    use super::*;
    use madsim::runtime::Runtime;

    /// Run job replay in madsim simulation.
    pub async fn run_in_simulation(replay_path: &str, seed: u64) -> Result<ReplayStats> {
        let runtime = Runtime::new();

        runtime.block_on(async {
            // Load replay
            let replay = JobReplaySystem::load(replay_path)?;

            // Create simulated environment
            let store = Arc::new(aspen_core::inmemory::DeterministicKeyValueStore::new());
            let manager = Arc::new(JobManager::new(store));
            manager.initialize().await?;

            // Run replay
            let mut runner = ReplayRunner::new(replay, manager);
            runner.run().await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_save_load() {
        let mut replay = JobReplaySystem::new(12345);

        // Record some events
        let job = JobSpec::new("test_job").payload(serde_json::json!({"test": true})).unwrap();
        replay.record_submission(&job, 1000);

        let job_id = JobId::new();
        replay.record_execution(job_id.clone(), "worker-1".to_string(), 1100);
        replay.record_completion(job_id, JobResult::success(serde_json::json!({})), 200, 1300);

        // Save and load
        let path = "/tmp/test_replay.json";
        replay.save(path).unwrap();

        let loaded = JobReplaySystem::load(path).unwrap();
        assert_eq!(loaded.seed, 12345);
        assert_eq!(loaded.events.len(), 3);
    }

    #[tokio::test]
    async fn test_deterministic_executor() {
        let mut executor = DeterministicJobExecutor::new(42);

        let job = Job::from_spec(JobSpec::new("test").payload(serde_json::json!({})).unwrap());

        let result = executor.execute(&job).await;
        assert!(result.is_success());

        // Verify deterministic behavior
        let duration1 = executor.get_execution_duration(&job.id);
        let duration2 = executor.get_execution_duration(&job.id);
        assert_eq!(duration1, duration2);
    }
}
