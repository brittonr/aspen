//! CI Pipeline test utilities for madsim-based distributed testing.
//!
//! This module provides `CiPipelineTester`, a high-level abstraction that extends
//! `JobWorkerTester` with CI pipeline-specific capabilities for testing end-to-end
//! pipeline execution, trigger handling, and failure scenarios.
//!
//! # Design Principles (Tiger Style)
//!
//! - **Bounded resources**: All operations respect MAX_* limits from aspen_core
//! - **Explicit types**: Uses PipelineRun, JobId for clear identification
//! - **Fail-fast**: All errors propagate immediately via Result
//! - **Deterministic**: Uses madsim RNG for reproducible tests
//!
//! # Example
//!
//! ```ignore
//! use aspen_testing::ci_pipeline_tester::{CiPipelineTester, CiPipelineTestConfig};
//!
//! #[madsim::test]
//! async fn test_pipeline_execution() {
//!     let config = CiPipelineTestConfig::default();
//!     let mut t = CiPipelineTester::new(3, "pipeline_test", config).await;
//!
//!     // Submit a pipeline
//!     let run = t.submit_pipeline(pipeline_config, context).await.unwrap();
//!
//!     // Wait for completion
//!     let run = t.wait_for_pipeline(&run.id, Duration::from_secs(60)).await.unwrap();
//!     assert_eq!(run.status, PipelineStatus::Success);
//!
//!     t.end();
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen_ci::config::types::ArtifactConfig;
use aspen_ci::config::types::IsolationMode;
use aspen_ci::config::types::JobConfig;
use aspen_ci::config::types::JobType;
use aspen_ci::config::types::PipelineConfig;
use aspen_ci::config::types::Priority;
use aspen_ci::config::types::StageConfig;
use aspen_ci::config::types::TriggerConfig;
use aspen_ci::orchestrator::PipelineContext;
use aspen_ci::orchestrator::PipelineOrchestrator;
use aspen_ci::orchestrator::PipelineOrchestratorConfig;
use aspen_ci::orchestrator::PipelineRun;
use aspen_ci::orchestrator::PipelineStatus;
use aspen_ci::trigger::ConfigFetcher;
use aspen_ci::trigger::PipelineStarter;
use aspen_ci::trigger::TriggerEvent;
use aspen_ci::trigger::TriggerService;
use aspen_core::inmemory::DeterministicKeyValueStore;
use aspen_core::simulation::SimulationArtifact;
use aspen_forge::identity::RepoId;
use aspen_jobs::JobManager;
use aspen_jobs::WorkflowManager;
use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use crate::madsim_tester::AspenRaftTester;
use crate::madsim_tester::TesterConfig;

/// Tiger Style: Fixed limits for CI pipeline testing.
const MAX_PIPELINE_RUNS: usize = 100;
const DEFAULT_PIPELINE_TIMEOUT_SECS: u64 = 300;

/// Configuration for CI pipeline test scenarios.
#[derive(Debug, Clone)]
pub struct CiPipelineTestConfig {
    /// Base Raft tester configuration.
    pub raft_config: TesterConfig,
    /// Pipeline orchestrator configuration.
    pub orchestrator_config: PipelineOrchestratorConfig,
    /// Enable auto-triggering.
    pub auto_trigger: bool,
}

impl Default for CiPipelineTestConfig {
    fn default() -> Self {
        Self {
            raft_config: TesterConfig::new(3, "ci_pipeline_test"),
            orchestrator_config: PipelineOrchestratorConfig {
                avoid_leader: false,       // In tests, we don't care about leader affinity
                resource_isolation: false, // cgroups not available in madsim
                ..Default::default()
            },
            auto_trigger: false,
        }
    }
}

impl CiPipelineTestConfig {
    /// Create a new config with specified node count and test name.
    pub fn new(node_count: usize, test_name: &str) -> Self {
        Self {
            raft_config: TesterConfig::new(node_count, test_name),
            ..Default::default()
        }
    }

    /// Set explicit seed for deterministic testing.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.raft_config.seed = Some(seed);
        self
    }

    /// Enable auto-triggering for trigger service tests.
    pub fn with_auto_trigger(mut self, enabled: bool) -> Self {
        self.auto_trigger = enabled;
        self
    }
}

/// Tracked pipeline execution event.
#[derive(Debug, Clone)]
pub struct PipelineExecutionEvent {
    /// Pipeline run ID.
    pub run_id: String,
    /// Repository ID.
    pub repo_id: RepoId,
    /// Pipeline name.
    pub pipeline_name: String,
    /// Node index where orchestrator ran.
    pub node_idx: usize,
    /// Start timestamp (relative to test start).
    pub started_at_ms: u64,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Final status.
    pub status: PipelineStatus,
}

/// Simulated pipeline execution tracker for deterministic testing.
#[derive(Debug)]
pub struct SimulatedPipelineTracker {
    /// Pipeline execution events.
    executed: Mutex<Vec<PipelineExecutionEvent>>,
    /// Test start time for relative timestamps.
    start_time: std::time::Instant,
}

impl Default for SimulatedPipelineTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedPipelineTracker {
    /// Create a new tracker.
    pub fn new() -> Self {
        Self {
            executed: Mutex::new(Vec::new()),
            start_time: std::time::Instant::now(),
        }
    }

    /// Record pipeline start.
    pub async fn record_start(&self, run_id: &str, repo_id: RepoId, pipeline_name: &str, node_idx: usize) -> u64 {
        let timestamp = self.start_time.elapsed().as_millis() as u64;
        let mut executed = self.executed.lock().await;
        executed.push(PipelineExecutionEvent {
            run_id: run_id.to_string(),
            repo_id,
            pipeline_name: pipeline_name.to_string(),
            node_idx,
            started_at_ms: timestamp,
            duration_ms: 0,
            status: PipelineStatus::Running,
        });
        timestamp
    }

    /// Record pipeline completion.
    pub async fn record_completion(&self, run_id: &str, status: PipelineStatus, duration_ms: u64) {
        let mut executed = self.executed.lock().await;
        if let Some(event) = executed.iter_mut().rev().find(|e| e.run_id == run_id) {
            event.duration_ms = duration_ms;
            event.status = status;
        }
    }

    /// Get all execution events.
    pub async fn executions(&self) -> Vec<PipelineExecutionEvent> {
        self.executed.lock().await.clone()
    }

    /// Get pipelines executed on a specific node.
    pub async fn pipelines_on_node(&self, node_idx: usize) -> Vec<String> {
        let executed = self.executed.lock().await;
        executed.iter().filter(|e| e.node_idx == node_idx).map(|e| e.run_id.clone()).collect()
    }
}

/// Mock config fetcher for testing that returns predefined configs.
pub struct MockConfigFetcher {
    /// Configs by repo ID hex string.
    configs: RwLock<HashMap<String, String>>,
}

impl MockConfigFetcher {
    /// Create a new mock fetcher.
    pub fn new() -> Self {
        Self {
            configs: RwLock::new(HashMap::new()),
        }
    }

    /// Set config for a repository.
    pub async fn set_config(&self, repo_id: &RepoId, config_content: String) {
        self.configs.write().await.insert(repo_id.to_hex(), config_content);
    }

    /// Remove config for a repository.
    pub async fn remove_config(&self, repo_id: &RepoId) {
        self.configs.write().await.remove(&repo_id.to_hex());
    }
}

impl Default for MockConfigFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfigFetcher for MockConfigFetcher {
    async fn fetch_config(
        &self,
        repo_id: &RepoId,
        _commit_hash: &[u8; 32],
        _config_path: &str,
    ) -> aspen_ci::error::Result<Option<String>> {
        let configs = self.configs.read().await;
        Ok(configs.get(&repo_id.to_hex()).cloned())
    }
}

/// Mock pipeline starter for testing that tracks started pipelines.
pub struct MockPipelineStarter {
    /// Started pipelines.
    started: Mutex<Vec<TriggerEvent>>,
    /// Counter for generating run IDs.
    counter: std::sync::atomic::AtomicU32,
}

impl MockPipelineStarter {
    /// Create a new mock starter.
    pub fn new() -> Self {
        Self {
            started: Mutex::new(Vec::new()),
            counter: std::sync::atomic::AtomicU32::new(0),
        }
    }

    /// Get started pipelines.
    pub async fn started_pipelines(&self) -> Vec<TriggerEvent> {
        self.started.lock().await.clone()
    }

    /// Get count of started pipelines.
    pub async fn started_count(&self) -> usize {
        self.started.lock().await.len()
    }
}

impl Default for MockPipelineStarter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelineStarter for MockPipelineStarter {
    async fn start_pipeline(&self, event: TriggerEvent) -> aspen_ci::error::Result<String> {
        let count = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.started.lock().await.push(event);
        Ok(format!("mock-run-{}", count))
    }
}

/// High-level tester for CI pipeline integration tests.
///
/// Wraps `AspenRaftTester` and adds CI-specific testing capabilities
/// including pipeline submission, trigger testing, and status tracking.
pub struct CiPipelineTester {
    /// Underlying Raft tester.
    raft: AspenRaftTester,
    /// Pipeline orchestrators per node (indexed by node idx).
    orchestrators: Vec<Arc<PipelineOrchestrator<DeterministicKeyValueStore>>>,
    /// Workflow managers per node.
    #[allow(dead_code)]
    workflow_managers: Vec<Arc<WorkflowManager<DeterministicKeyValueStore>>>,
    /// Job managers per node.
    #[allow(dead_code)]
    job_managers: Vec<Arc<JobManager<DeterministicKeyValueStore>>>,
    /// Shared KV stores per node.
    #[allow(dead_code)]
    stores: Vec<Arc<DeterministicKeyValueStore>>,
    /// Pipeline execution tracker.
    tracker: Arc<SimulatedPipelineTracker>,
    /// Submitted pipeline runs for tracking.
    submitted_runs: Vec<String>,
    /// Test configuration.
    #[allow(dead_code)]
    config: CiPipelineTestConfig,
    /// Trigger services per node (if auto_trigger enabled).
    #[allow(dead_code)]
    trigger_services: Vec<Option<Arc<TriggerService>>>,
    /// Mock config fetcher (shared across nodes).
    config_fetcher: Arc<MockConfigFetcher>,
}

impl CiPipelineTester {
    /// Create a new CI pipeline tester with n nodes.
    pub async fn new(n: usize, test_name: &str, config: CiPipelineTestConfig) -> Self {
        assert!(n > 0 && n <= 64, "node_count must be between 1 and 64");

        // Create Raft tester with updated config
        let mut raft_config = config.raft_config.clone();
        raft_config.node_count = n;
        raft_config.test_name = test_name.to_string();
        let raft = AspenRaftTester::with_config(raft_config).await;

        // Create shared infrastructure
        let tracker = Arc::new(SimulatedPipelineTracker::new());
        let config_fetcher = Arc::new(MockConfigFetcher::new());
        let mut orchestrators: Vec<Arc<PipelineOrchestrator<DeterministicKeyValueStore>>> = Vec::with_capacity(n);
        let mut workflow_managers: Vec<Arc<WorkflowManager<DeterministicKeyValueStore>>> = Vec::with_capacity(n);
        let mut job_managers: Vec<Arc<JobManager<DeterministicKeyValueStore>>> = Vec::with_capacity(n);
        let mut stores: Vec<Arc<DeterministicKeyValueStore>> = Vec::with_capacity(n);
        let mut trigger_services: Vec<Option<Arc<TriggerService>>> = Vec::with_capacity(n);

        // Initialize orchestrator infrastructure for each node
        for _i in 0..n {
            // Create the KV store for this node
            let store = DeterministicKeyValueStore::new();
            stores.push(store.clone());

            // Create job manager
            let job_manager = Arc::new(JobManager::new(store.clone()));
            job_manager.initialize().await.expect("failed to initialize job manager");
            job_managers.push(job_manager.clone());

            // Create workflow manager
            let workflow_manager = Arc::new(WorkflowManager::new(job_manager.clone(), store.clone()));
            workflow_managers.push(workflow_manager.clone());

            // Create pipeline orchestrator
            let orchestrator = Arc::new(PipelineOrchestrator::new(
                config.orchestrator_config.clone(),
                workflow_manager.clone(),
                job_manager.clone(),
                None, // No blob store for tests
                store.clone(),
            ));

            // Initialize the orchestrator (sets up job completion callbacks)
            orchestrator.init().await;
            orchestrators.push(orchestrator);

            // Trigger service is created but not started in this basic version
            // Full trigger testing would require gossip integration
            trigger_services.push(None);
        }

        Self {
            raft,
            orchestrators,
            workflow_managers,
            job_managers,
            stores,
            tracker,
            submitted_runs: Vec::new(),
            config,
            trigger_services,
            config_fetcher,
        }
    }

    /// Get the underlying Raft tester for advanced operations.
    pub fn raft(&mut self) -> &mut AspenRaftTester {
        &mut self.raft
    }

    /// Get the pipeline execution tracker.
    pub fn tracker(&self) -> &SimulatedPipelineTracker {
        &self.tracker
    }

    /// Get the mock config fetcher for setting up test configs.
    pub fn config_fetcher(&self) -> &MockConfigFetcher {
        &self.config_fetcher
    }

    /// Submit a pipeline to a specific node.
    pub async fn submit_pipeline(
        &mut self,
        node_idx: usize,
        config: PipelineConfig,
        context: PipelineContext,
    ) -> Result<PipelineRun> {
        assert!(node_idx < self.orchestrators.len(), "invalid node index");
        assert!(self.submitted_runs.len() < MAX_PIPELINE_RUNS, "exceeded MAX_PIPELINE_RUNS");

        let orchestrator = &self.orchestrators[node_idx];

        // Track start
        self.tracker.record_start("pending", context.repo_id, &config.name, node_idx).await;

        // Execute the pipeline
        let run = orchestrator.execute(config, context).await?;

        // Update tracking with real run ID
        self.tracker.record_start(&run.id, run.context.repo_id, &run.pipeline_name, node_idx).await;
        self.submitted_runs.push(run.id.clone());

        Ok(run)
    }

    /// Submit a pipeline to the cluster (uses node 0).
    pub async fn submit_pipeline_to_cluster(
        &mut self,
        config: PipelineConfig,
        context: PipelineContext,
    ) -> Result<PipelineRun> {
        self.submit_pipeline(0, config, context).await
    }

    /// Wait for a specific pipeline to complete.
    pub async fn wait_for_pipeline(&self, run_id: &str, timeout: Duration) -> Result<PipelineRun> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(100);

        while start.elapsed() < timeout {
            // Check all orchestrators for the run
            for orchestrator in &self.orchestrators {
                if let Some(run) = orchestrator.get_run(run_id).await
                    && run.status.is_terminal()
                {
                    // Record completion
                    let duration = start.elapsed().as_millis() as u64;
                    self.tracker.record_completion(run_id, run.status, duration).await;
                    return Ok(run);
                }
            }
            madsim::time::sleep(poll_interval).await;
        }

        anyhow::bail!("timeout waiting for pipeline {} to complete", run_id)
    }

    /// Wait for all submitted pipelines to complete.
    pub async fn wait_for_all_pipelines(&self, timeout: Duration) -> Result<Vec<PipelineRun>> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(200);
        let run_ids = self.submitted_runs.clone();

        while start.elapsed() < timeout {
            let mut all_complete = true;
            let mut completed_runs = Vec::new();

            for run_id in &run_ids {
                let mut found = false;
                for orchestrator in &self.orchestrators {
                    if let Some(run) = orchestrator.get_run(run_id).await {
                        if run.status.is_terminal() {
                            completed_runs.push(run);
                            found = true;
                            break;
                        } else {
                            all_complete = false;
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    all_complete = false;
                }
            }

            if all_complete && completed_runs.len() == run_ids.len() {
                return Ok(completed_runs);
            }

            madsim::time::sleep(poll_interval).await;
        }

        anyhow::bail!("timeout waiting for all {} pipelines to complete", run_ids.len())
    }

    /// Get a pipeline run from any node.
    pub async fn get_run(&self, run_id: &str) -> Option<PipelineRun> {
        for orchestrator in &self.orchestrators {
            if let Some(run) = orchestrator.get_run(run_id).await {
                return Some(run);
            }
        }
        None
    }

    /// Cancel a pipeline run.
    pub async fn cancel_pipeline(&self, node_idx: usize, run_id: &str) -> Result<()> {
        assert!(node_idx < self.orchestrators.len(), "invalid node index");
        self.orchestrators[node_idx].cancel(run_id).await?;
        Ok(())
    }

    /// List all runs for a repository.
    pub async fn list_runs(&self, node_idx: usize, repo_id: &RepoId) -> Vec<PipelineRun> {
        assert!(node_idx < self.orchestrators.len(), "invalid node index");
        self.orchestrators[node_idx].list_runs(repo_id).await
    }

    /// Get active run count on a node.
    pub async fn active_run_count(&self, node_idx: usize) -> usize {
        assert!(node_idx < self.orchestrators.len(), "invalid node index");
        self.orchestrators[node_idx].active_run_count().await
    }

    /// Crash a node (delegates to Raft tester).
    pub async fn crash_node(&mut self, node_idx: usize) {
        self.raft.crash_node(node_idx).await;
    }

    /// Restart a node (delegates to Raft tester).
    pub async fn restart_node(&mut self, node_idx: usize) {
        self.raft.restart_node(node_idx).await;
    }

    /// Disconnect a node from the network.
    pub fn disconnect(&mut self, node_idx: usize) {
        self.raft.disconnect(node_idx);
    }

    /// Reconnect a node to the network.
    pub fn connect(&mut self, node_idx: usize) {
        self.raft.connect(node_idx);
    }

    /// Check for exactly one leader in the Raft cluster.
    pub async fn check_one_leader(&mut self) -> Option<usize> {
        self.raft.check_one_leader().await
    }

    /// End the test and return simulation artifact.
    pub fn end(self) -> SimulationArtifact {
        self.raft.end()
    }
}

/// Helper to create a simple test job config.
fn test_job(name: &str, command: &str) -> JobConfig {
    JobConfig {
        name: name.to_string(),
        job_type: JobType::Shell,
        command: Some(command.to_string()),
        args: vec![],
        env: HashMap::new(),
        working_dir: None,
        flake_url: None,
        flake_attr: None,
        binary_hash: None,
        timeout_secs: 60,
        isolation: IsolationMode::None,
        cache_key: None,
        artifacts: vec![],
        depends_on: vec![],
        retry_count: 0,
        allow_failure: false,
        tags: vec![],
        upload_result: true,
    }
}

/// Helper to create a simple test pipeline config.
pub fn simple_test_pipeline(name: &str) -> PipelineConfig {
    PipelineConfig {
        name: name.to_string(),
        description: None,
        stages: vec![StageConfig {
            name: "test".to_string(),
            jobs: vec![test_job("test-job", "echo hello")],
            parallel: false,
            depends_on: vec![],
            when: None,
        }],
        triggers: TriggerConfig::default(),
        artifacts: ArtifactConfig::default(),
        env: HashMap::new(),
        timeout_secs: DEFAULT_PIPELINE_TIMEOUT_SECS,
        priority: Priority::Normal,
    }
}

/// Helper to create a test pipeline context.
pub fn test_pipeline_context(repo_name: &str) -> PipelineContext {
    PipelineContext {
        repo_id: RepoId::from_hash(blake3::hash(repo_name.as_bytes())),
        commit_hash: [1u8; 32],
        ref_name: "refs/heads/main".to_string(),
        triggered_by: "test".to_string(),
        env: HashMap::new(),
        checkout_dir: None,
    }
}

/// Helper to create a multi-stage test pipeline.
pub fn multi_stage_test_pipeline(name: &str, stage_count: usize) -> PipelineConfig {
    let mut stages: Vec<StageConfig> = Vec::with_capacity(stage_count);

    for i in 0..stage_count {
        let depends_on = if i == 0 {
            vec![]
        } else {
            vec![stages[i - 1].name.clone()]
        };

        stages.push(StageConfig {
            name: format!("stage-{}", i),
            jobs: vec![test_job(&format!("job-{}", i), &format!("echo stage {}", i))],
            parallel: false,
            depends_on,
            when: None,
        });
    }

    PipelineConfig {
        name: name.to_string(),
        description: None,
        stages,
        triggers: TriggerConfig::default(),
        artifacts: ArtifactConfig::default(),
        env: HashMap::new(),
        timeout_secs: DEFAULT_PIPELINE_TIMEOUT_SECS,
        priority: Priority::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_pipeline_config() {
        let config = simple_test_pipeline("test");
        assert_eq!(config.name, "test");
        assert_eq!(config.stages.len(), 1);
        assert_eq!(config.stages[0].jobs.len(), 1);
    }

    #[test]
    fn test_multi_stage_pipeline_config() {
        let config = multi_stage_test_pipeline("test", 3);
        assert_eq!(config.name, "test");
        assert_eq!(config.stages.len(), 3);

        // Check dependencies
        assert!(config.stages[0].depends_on.is_empty());
        assert_eq!(config.stages[1].depends_on, vec!["stage-0"]);
        assert_eq!(config.stages[2].depends_on, vec!["stage-1"]);
    }

    #[test]
    fn test_pipeline_context() {
        let context = test_pipeline_context("test-repo");
        assert_eq!(context.ref_name, "refs/heads/main");
        assert_eq!(context.triggered_by, "test");
    }
}
