//! Example demonstrating deterministic job replay with madsim simulation.
//!
//! This example shows how to record job execution sequences and replay them
//! deterministically for testing and debugging distributed job scenarios.

use std::sync::Arc;
use std::time::Duration;

use aspen_jobs::DeterministicJobExecutor;
use aspen_jobs::Job;
use aspen_jobs::JobManager;
use aspen_jobs::JobReplaySystem;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::Priority;
use aspen_jobs::ReplayConfig;
use aspen_jobs::ReplayRunner;
use aspen_jobs::Worker;
use aspen_jobs::WorkerPool;
use aspen_testing::DeterministicKeyValueStore;
use async_trait::async_trait;
use rand::Rng;
use tokio::time::sleep;
use tracing::Level;
use tracing::info;
use tracing::warn;

/// Worker that records its execution for replay.
struct RecordingWorker {
    worker_id: String,
    replay_system: Arc<tokio::sync::RwLock<JobReplaySystem>>,
}

#[async_trait]
impl Worker for RecordingWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let _timestamp = chrono::Utc::now().timestamp_millis() as u64;

        // Record execution start
        {
            let mut replay = self.replay_system.write().await;
            replay.record_execution(job.id.clone(), self.worker_id.clone());
        }

        info!("🎬 Worker {} executing job {}", self.worker_id, job.id);

        // Simulate work with variable duration
        let duration = {
            let mut rng = rand::thread_rng();
            rng.random_range(100..500)
        };
        sleep(Duration::from_millis(duration)).await;

        // Simulate occasional failures
        let should_fail = {
            let mut rng = rand::thread_rng();
            rng.random_range(0.0..1.0) < 0.15 // 15% failure rate
        };

        let result = if should_fail {
            let error = format!("Worker {} encountered error processing job", self.worker_id);
            warn!("❌ {}", error);

            // Record failure
            {
                let mut replay = self.replay_system.write().await;
                replay.record_failure(job.id.clone(), error.clone());
            }

            JobResult::failure(error)
        } else {
            info!("✅ Worker {} completed job {}", self.worker_id, job.id);

            let result = JobResult::success(serde_json::json!({
                "worker": self.worker_id,
                "duration_ms": duration,
            }));

            // Record completion
            {
                let mut replay = self.replay_system.write().await;
                replay.record_completion(job.id.clone(), result.clone(), duration);
            }

            result
        };

        result
    }

    fn job_types(&self) -> Vec<String> {
        vec!["compute".to_string(), "io".to_string(), "network".to_string()]
    }
}

/// Run original job execution and record events.
async fn run_original_execution(
    manager: Arc<JobManager<Arc<DeterministicKeyValueStore>>>,
    replay_system: Arc<tokio::sync::RwLock<JobReplaySystem>>,
) -> anyhow::Result<Vec<aspen_jobs::JobId>> {
    info!("\n📹 Recording Original Execution\n");

    // Create worker pool with recording workers
    let pool = WorkerPool::with_manager(manager.clone());

    for i in 1..=3 {
        let worker_id = format!("worker-{}", i);
        pool.register_handler("compute", RecordingWorker {
            worker_id: worker_id.clone(),
            replay_system: replay_system.clone(),
        })
        .await?;

        pool.register_handler("io", RecordingWorker {
            worker_id: worker_id.clone(),
            replay_system: replay_system.clone(),
        })
        .await?;

        pool.register_handler("network", RecordingWorker {
            worker_id,
            replay_system: replay_system.clone(),
        })
        .await?;
    }

    pool.start(3).await?;

    // Submit jobs and record submissions
    let job_types = vec!["compute", "io", "network"];
    let priorities = vec![Priority::Critical, Priority::High, Priority::Normal];
    let mut job_ids = Vec::new();

    for i in 0..10 {
        let job_type = job_types[i % job_types.len()];
        let priority = priorities[i % priorities.len()].clone();

        let job_spec = JobSpec::new(job_type).priority(priority).payload(serde_json::json!({
            "task_id": i,
            "data_size": 1000 * (i + 1),
        }))?;

        // Record submission
        {
            let _timestamp = chrono::Utc::now().timestamp_millis() as u64;
            let mut replay = replay_system.write().await;
            replay.record_submission(&job_spec);
        }

        let job_id = manager.submit(job_spec).await?;
        job_ids.push(job_id.clone());
        info!("  Submitted job {}: {} ({})", i, job_id, job_type);

        // Small delay between submissions
        if i % 3 == 2 {
            sleep(Duration::from_millis(100)).await;
        }
    }

    // Simulate some chaos events
    info!("\n💥 Injecting Chaos Events\n");

    sleep(Duration::from_millis(500)).await;

    // Record network partition
    {
        let _timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let mut replay = replay_system.write().await;
        replay.record_partition(vec!["worker-1".to_string(), "worker-2".to_string()]);
        info!("  Network partition: worker-1 <-> worker-2");
    }

    sleep(Duration::from_millis(300)).await;

    // Record node crash
    {
        let _timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let mut replay = replay_system.write().await;
        replay.record_crash("worker-3".to_string());
        info!("  Node crash: worker-3");
    }

    // Wait for processing
    sleep(Duration::from_secs(2)).await;

    // Shutdown
    pool.shutdown().await?;

    Ok(job_ids)
}

/// Run deterministic replay of recorded execution.
async fn run_deterministic_replay(replay_path: &str, seed: u64) -> anyhow::Result<()> {
    info!("\n🔄 Running Deterministic Replay (seed: {})\n", seed);

    // Load replay
    let replay_system = JobReplaySystem::load(replay_path, "replay-node")?;
    info!("  Loaded {} events from replay", replay_system.events().len());

    // Create new environment for replay
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = Arc::new(JobManager::new(store));
    manager.initialize().await?;

    // Configure replay
    let config = ReplayConfig {
        speed: 2.0, // 2x speed
        chaos_enabled: true,
        failure_rate: 0.1,
        network_delay: (10, 50),
        is_deterministic: true,
        max_duration_ms: 10_000,
    };

    let replay_with_config = replay_system.with_config(config.clone());

    // Run replay
    let mut runner = ReplayRunner::new(replay_with_config, manager.clone(), "replay-node");
    let stats = runner.run().await?;

    info!("\n📊 Replay Statistics:");
    info!("  Jobs submitted: {}", stats.jobs_submitted);
    info!("  Jobs started: {}", stats.jobs_started);
    info!("  Jobs completed: {}", stats.jobs_completed);
    info!("  Jobs failed: {}", stats.jobs_failed);
    info!("  Network partitions: {}", stats.network_partitions);
    info!("  Node crashes: {}", stats.node_crashes);
    info!("  Total execution time: {} ms", stats.total_execution_time_ms);
    info!("  Replay duration: {} ms", stats.duration_ms);

    Ok(())
}

/// Demonstrate deterministic executor behavior.
async fn demonstrate_deterministic_executor() -> anyhow::Result<()> {
    info!("\n🎯 Demonstrating Deterministic Executor\n");

    // Create two executors with same seed
    let mut executor1 = DeterministicJobExecutor::new(42, "node-1");
    let mut executor2 = DeterministicJobExecutor::new(42, "node-2");

    // Create test job spec
    let job_spec = JobSpec::new("test").payload(serde_json::json!({"test": true}))?;

    // Create minimal Job for testing
    let job = Job::from_spec(job_spec);

    info!("  Executing same job with two executors (same seed)...");

    // Execute with both executors
    let result1 = executor1.execute(&job).await;
    let result2 = executor2.execute(&job).await;

    // Results should be identical due to deterministic execution
    info!("  Executor 1 result: {:?}", result1.is_success());
    info!("  Executor 2 result: {:?}", result2.is_success());
    assert_eq!(result1.is_success(), result2.is_success());

    // Create executor with different seed
    let mut executor3 = DeterministicJobExecutor::new(99, "node-3").with_failures(0.5);

    info!("\n  Executing with different seed and failure injection...");
    for i in 0..5 {
        let test_spec = JobSpec::new("test").payload(serde_json::json!({"iteration": i}))?;
        let test_job = Job::from_spec(test_spec);

        let result = executor3.execute(&test_job).await;
        info!(
            "    Iteration {}: {}",
            i,
            if result.is_success() {
                "Success"
            } else {
                "Failed (injected)"
            }
        );
    }

    // Show execution history
    info!("\n  Execution History:");
    for (job_id, record) in executor3.history() {
        info!("    Job {}: retries={}", &job_id.to_string()[..8], record.retries);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 Starting Deterministic Job Replay Demo\n");

    // Create initial environment
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = Arc::new(JobManager::new(store));
    manager.initialize().await?;

    // Create replay system
    let seed = 12345;
    let replay_system = Arc::new(tokio::sync::RwLock::new(JobReplaySystem::new(seed, "example-node")));

    // Run original execution with recording
    let job_ids = run_original_execution(manager.clone(), replay_system.clone()).await?;

    info!("\n📋 Job Execution Summary:");
    for job_id in &job_ids {
        if let Some(job) = manager.get_job(job_id).await? {
            let status_icon = match job.status {
                aspen_jobs::JobStatus::Completed => "✅",
                aspen_jobs::JobStatus::Failed => "❌",
                _ => "⏳",
            };
            info!("  {} Job {}: {:?}", status_icon, &job_id.to_string()[..8], job.status);
        }
    }

    // Save replay to file
    let replay_path = "/tmp/job_replay.json";
    {
        let replay = replay_system.read().await;
        replay.save(replay_path)?;
        info!("\n💾 Saved replay to {}", replay_path);
    }

    // Demonstrate replay with different configurations
    info!("\n🎮 Replay Demonstrations:");

    // First replay - normal speed
    run_deterministic_replay(replay_path, seed).await?;

    // Second replay - same seed should produce same results
    info!("\n🔁 Running replay again with same seed (should be identical):");
    run_deterministic_replay(replay_path, seed).await?;

    // Third replay - different seed
    info!("\n🎲 Running replay with different seed:");
    run_deterministic_replay(replay_path, 99999).await?;

    // Demonstrate deterministic executor
    demonstrate_deterministic_executor().await?;

    info!("\n✅ Demo complete!");

    Ok(())
}
