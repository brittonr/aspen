//! Example demonstrating CRDT-based job progress tracking.
//!
//! This example shows how multiple workers can update job progress
//! concurrently using CRDTs for conflict-free eventual consistency.

use std::sync::Arc;
use std::time::Duration;

use aspen_jobs::CrdtProgressTracker;
use aspen_jobs::Job;
use aspen_jobs::JobManager;
use aspen_jobs::JobProgress;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::ProgressSyncManager;
use aspen_jobs::ProgressUpdate;
use aspen_jobs::Worker;
use aspen_jobs::WorkerPool;
use aspen_testing::DeterministicKeyValueStore;
use async_trait::async_trait;
use rand::Rng;
use tokio::time::interval;
use tokio::time::sleep;
use tracing::Level;
use tracing::info;

/// Worker that reports progress using CRDTs.
struct ProgressReportingWorker {
    worker_id: String,
    progress_tracker: Arc<CrdtProgressTracker>,
}

#[async_trait]
impl Worker for ProgressReportingWorker {
    async fn execute(&self, job: Job) -> JobResult {
        info!("🚀 Worker {} starting job {}", self.worker_id, job.id);

        // Initialize progress
        self.progress_tracker
            .update_progress(&job.id, ProgressUpdate::SetStep("initializing".to_string()))
            .await
            .unwrap();

        // Simulate multi-step processing with progress updates
        let steps = vec![
            ("parsing", 10),
            ("validation", 20),
            ("processing", 50),
            ("optimization", 70),
            ("finalization", 90),
            ("cleanup", 100),
        ];

        for (step_name, percentage) in steps {
            // Random processing time
            let delay = {
                let mut rng = rand::thread_rng();
                rng.gen_range(100..500)
            };
            sleep(Duration::from_millis(delay)).await;

            // Update progress with multiple CRDT operations
            let updates = vec![
                ProgressUpdate::SetStep(step_name.to_string()),
                ProgressUpdate::SetPercentage(percentage),
                ProgressUpdate::CompleteStep(step_name.to_string()),
            ];

            self.progress_tracker.update_progress(&job.id, ProgressUpdate::Batch(updates)).await.unwrap();

            // Occasionally add metrics
            if percentage % 30 == 0 {
                let throughput = {
                    let mut rng = rand::thread_rng();
                    rng.gen_range(100.0..1000.0)
                };

                self.progress_tracker
                    .update_progress(&job.id, ProgressUpdate::SetMetric {
                        key: format!("{}_throughput", step_name),
                        value: throughput,
                    })
                    .await
                    .unwrap();
            }

            // Simulate occasional errors/warnings
            let should_warn = {
                let mut rng = rand::thread_rng();
                rng.gen_range(0.0..1.0) < 0.2
            };

            if should_warn {
                self.progress_tracker.update_progress(&job.id, ProgressUpdate::IncrementWarnings(1)).await.unwrap();
            }

            info!("  Worker {} - Step: {} ({}%)", self.worker_id, step_name, percentage);
        }

        // Final success
        JobResult::success(serde_json::json!({
            "worker": self.worker_id,
            "steps_completed": 6,
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["data_pipeline".to_string(), "batch_process".to_string()]
    }
}

/// Monitor that displays progress from all workers.
async fn progress_monitor(tracker: Arc<CrdtProgressTracker>, job_ids: Vec<aspen_jobs::JobId>) {
    let mut ticker = interval(Duration::from_millis(500));

    for _ in 0..20 {
        ticker.tick().await;

        info!("\n📊 Progress Update:");
        for job_id in &job_ids {
            if let Some(progress) = tracker.get_progress(job_id).await {
                info!(
                    "  Job {}: {}% - Step: {} - Completed: {:?}",
                    &job_id.to_string()[..8],
                    progress.percentage,
                    progress.current_step,
                    progress.completed_steps
                );

                if progress.error_count > 0 || progress.warning_count > 0 {
                    info!("    ⚠️  Errors: {}, Warnings: {}", progress.error_count, progress.warning_count);
                }

                if !progress.metrics.is_empty() {
                    info!("    📈 Metrics: {:?}", progress.metrics);
                }
            }
        }
    }
}

/// Simulate CRDT synchronization between nodes.
async fn sync_simulation(sync_managers: Vec<Arc<ProgressSyncManager>>) {
    let mut ticker = interval(Duration::from_secs(1));

    for _ in 0..5 {
        ticker.tick().await;

        info!("\n🔄 Syncing progress between nodes...");
        for manager in &sync_managers {
            manager.sync_all().await.unwrap();
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 Starting CRDT Progress Tracking Demo\n");

    // Create store and manager
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = Arc::new(JobManager::new(store.clone()));
    manager.initialize().await?;

    // Create progress trackers for different nodes
    let tracker_node1 = Arc::new(CrdtProgressTracker::new("node-1".to_string()));
    let tracker_node2 = Arc::new(CrdtProgressTracker::new("node-2".to_string()));
    let tracker_node3 = Arc::new(CrdtProgressTracker::new("node-3".to_string()));

    // Create sync managers
    let sync_manager1 = Arc::new(ProgressSyncManager::new(tracker_node1.clone()));
    let sync_manager2 = Arc::new(ProgressSyncManager::new(tracker_node2.clone()));
    let sync_manager3 = Arc::new(ProgressSyncManager::new(tracker_node3.clone()));

    // Add peers for synchronization
    sync_manager1.add_peer("node-2".to_string()).await;
    sync_manager1.add_peer("node-3".to_string()).await;
    sync_manager2.add_peer("node-1".to_string()).await;
    sync_manager2.add_peer("node-3".to_string()).await;
    sync_manager3.add_peer("node-1".to_string()).await;
    sync_manager3.add_peer("node-2".to_string()).await;

    // Create worker pools with progress tracking
    let pool1 = WorkerPool::with_manager(manager.clone());
    pool1
        .register_handler("data_pipeline", ProgressReportingWorker {
            worker_id: "worker-1".to_string(),
            progress_tracker: tracker_node1.clone(),
        })
        .await?;

    let pool2 = WorkerPool::with_manager(manager.clone());
    pool2
        .register_handler("data_pipeline", ProgressReportingWorker {
            worker_id: "worker-2".to_string(),
            progress_tracker: tracker_node2.clone(),
        })
        .await?;

    let pool3 = WorkerPool::with_manager(manager.clone());
    pool3
        .register_handler("batch_process", ProgressReportingWorker {
            worker_id: "worker-3".to_string(),
            progress_tracker: tracker_node3.clone(),
        })
        .await?;

    // Start worker pools
    pool1.start(1).await?;
    pool2.start(1).await?;
    pool3.start(1).await?;

    info!("📝 Submitting jobs for distributed processing...\n");

    // Submit jobs
    let mut job_ids = Vec::new();
    for i in 0..3 {
        let job_type = if i < 2 { "data_pipeline" } else { "batch_process" };

        let job = JobSpec::new(job_type).payload(serde_json::json!({
            "dataset_id": format!("dataset_{}", i),
            "records": 10000 * (i + 1),
        }))?;

        let job_id = manager.submit(job).await?;
        job_ids.push(job_id.clone());
        info!("  Submitted job: {} (type: {})", job_id, job_type);
    }

    // Start monitoring and sync tasks
    let monitor_handle = tokio::spawn(progress_monitor(tracker_node1.clone(), job_ids.clone()));
    let sync_handle =
        tokio::spawn(sync_simulation(vec![sync_manager1.clone(), sync_manager2.clone(), sync_manager3.clone()]));

    // Wait for processing
    sleep(Duration::from_secs(8)).await;

    info!("\n🔄 Demonstrating CRDT Merge Behavior\n");

    // Demonstrate CRDT merge
    info!("Getting CRDT states from all nodes...");
    let states1 = tracker_node1.get_all_states().await;
    let states2 = tracker_node2.get_all_states().await;
    let states3 = tracker_node3.get_all_states().await;

    info!("  Node 1 has {} job states", states1.len());
    info!("  Node 2 has {} job states", states2.len());
    info!("  Node 3 has {} job states", states3.len());

    // Merge all states to node 1
    info!("\nMerging all states to Node 1...");
    for state in states2 {
        tracker_node1.merge_remote(state).await?;
    }
    for state in states3 {
        tracker_node1.merge_remote(state).await?;
    }

    info!("\n📋 Final Consolidated Progress (After CRDT Merge):\n");
    for job_id in &job_ids {
        if let Some(progress) = tracker_node1.get_progress(job_id).await {
            info!("Job {} Final State:", &job_id.to_string()[..8]);
            info!("  Progress: {}%", progress.percentage);
            info!("  Current Step: {}", progress.current_step);
            info!("  Completed Steps: {:?}", progress.completed_steps);
            info!("  Warnings: {}", progress.warning_count);
            info!("  Metrics: {} entries", progress.metrics.len());
            info!("");
        }
    }

    // Demonstrate subscription
    info!("📡 Demonstrating Progress Subscription:\n");
    if let Some(job_id) = job_ids.first() {
        let subscription = tracker_node1.subscribe(job_id).await;

        for _ in 0..3 {
            if let Some(progress) = subscription.poll().await {
                info!("  Subscription update - Job {}: {}%", &job_id.to_string()[..8], progress.percentage);
            }
            sleep(Duration::from_millis(500)).await;
        }
    }

    // Demonstrate garbage collection
    info!("\n🗑️  Garbage Collection Demo:\n");
    let before_gc = tracker_node1.get_all_states().await.len();
    info!("  States before GC: {}", before_gc);

    let collected = tracker_node1.gc(5000).await; // GC entries older than 5 seconds
    info!("  Collected {} old entries", collected);

    let after_gc = tracker_node1.get_all_states().await.len();
    info!("  States after GC: {}", after_gc);

    // Wait for tasks
    monitor_handle.await?;
    sync_handle.await?;

    // Shutdown
    info!("\n🛑 Shutting down...");
    pool1.shutdown().await?;
    pool2.shutdown().await?;
    pool3.shutdown().await?;

    info!("✅ Demo complete!");

    Ok(())
}
