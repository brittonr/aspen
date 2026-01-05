//! Example demonstrating P2P locality-based job affinity.
//!
//! This example shows how jobs can be routed to workers based on:
//! - Network proximity (closest node)
//! - Data locality (where blobs are stored)
//! - Worker capabilities (tags)
//! - Load balancing (least loaded)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_core::inmemory::DeterministicKeyValueStore;
use aspen_jobs::AffinityJobManager;
use aspen_jobs::AffinityStrategy;
use aspen_jobs::Job;
use aspen_jobs::JobAffinity;
use aspen_jobs::JobManager;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::Worker;
use aspen_jobs::WorkerMetadata;
use aspen_jobs::WorkerPool;
use async_trait::async_trait;
use iroh::PublicKey as NodeId;
use rand;
use tokio::time::sleep;
use tracing::Level;
use tracing::info;

/// Data processing worker that tracks where it runs.
struct DataProcessor {
    worker_id: String,
    node_id: NodeId,
}

impl DataProcessor {
    fn new(worker_id: String) -> Self {
        Self {
            worker_id,
            node_id: iroh::SecretKey::generate(&mut rand::thread_rng()).public(),
        }
    }
}

#[async_trait]
impl Worker for DataProcessor {
    async fn execute(&self, job: Job) -> JobResult {
        info!(
            "🔄 Worker {} (node: {}) processing job {}",
            self.worker_id,
            self.node_id.to_string().chars().take(8).collect::<String>(),
            job.id
        );

        // Extract affinity info if present
        if let Some(affinity_str) = job.spec.payload.get("__affinity") {
            if let Ok(affinity) = serde_json::from_value::<JobAffinity>(affinity_str.clone()) {
                info!("  Affinity strategy: {:?}", affinity.strategy);
            }
        }

        // Check if we have the data locally (for data locality demo)
        if let Some(blob_hash) = job.spec.payload.get("blob_hash") {
            info!("  Processing blob: {}", blob_hash);
        }

        // Simulate processing
        sleep(Duration::from_millis(200)).await;

        JobResult::success(serde_json::json!({
            "processed_by": self.worker_id,
            "node_id": self.node_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["process_data".to_string()]
    }
}

/// ML worker with GPU capability.
struct MLWorker {
    worker_id: String,
}

#[async_trait]
impl Worker for MLWorker {
    async fn execute(&self, job: Job) -> JobResult {
        info!("🤖 ML Worker {} processing job {}", self.worker_id, job.id);

        // Simulate ML processing
        sleep(Duration::from_millis(500)).await;

        JobResult::success(serde_json::json!({
            "model": "neural_net_v2",
            "accuracy": 0.95,
            "processed_by": self.worker_id
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["ml_inference".to_string()]
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 Starting P2P Affinity Job Demo\n");

    // Create store and managers
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = Arc::new(JobManager::new(store.clone()));
    let affinity_manager = AffinityJobManager::new(manager.clone());

    // Initialize job system
    manager.initialize().await?;

    // Create simulated P2P network with 3 nodes
    let node1 = iroh::SecretKey::generate(&mut rand::thread_rng()).public();
    let node2 = iroh::SecretKey::generate(&mut rand::thread_rng()).public();
    let node3 = iroh::SecretKey::generate(&mut rand::thread_rng()).public();

    info!("📡 Simulated P2P Network:");
    info!("  Node 1: {}", node1.to_string().chars().take(8).collect::<String>());
    info!("  Node 2: {}", node2.to_string().chars().take(8).collect::<String>());
    info!("  Node 3: {}", node3.to_string().chars().take(8).collect::<String>());

    // Register worker metadata with different characteristics
    let mut worker1_metadata = WorkerMetadata {
        id: "worker-1".to_string(),
        node_id: node1,
        tags: vec!["fast".to_string(), "ssd".to_string()],
        region: Some("us-west".to_string()),
        load: 0.2,
        local_blobs: vec!["blob_abc123".to_string()],
        latencies: HashMap::new(),
        local_shards: vec![],
    };

    // Worker 1 has low latency to node2
    worker1_metadata.latencies.insert(node2, 10);
    worker1_metadata.latencies.insert(node3, 50);
    affinity_manager.update_worker_metadata(worker1_metadata).await;

    let mut worker2_metadata = WorkerMetadata {
        id: "worker-2".to_string(),
        node_id: node2,
        tags: vec!["gpu".to_string(), "ml".to_string()],
        region: Some("us-east".to_string()),
        load: 0.7,
        local_blobs: vec!["blob_def456".to_string()],
        latencies: HashMap::new(),
        local_shards: vec![],
    };

    // Worker 2 has low latency to node3
    worker2_metadata.latencies.insert(node1, 30);
    worker2_metadata.latencies.insert(node3, 15);
    affinity_manager.update_worker_metadata(worker2_metadata).await;

    let mut worker3_metadata = WorkerMetadata {
        id: "worker-3".to_string(),
        node_id: node3,
        tags: vec!["cpu".to_string(), "batch".to_string()],
        region: Some("eu-central".to_string()),
        load: 0.4,
        local_blobs: vec!["blob_ghi789".to_string()],
        latencies: HashMap::new(),
        local_shards: vec![],
    };

    worker3_metadata.latencies.insert(node1, 100);
    worker3_metadata.latencies.insert(node2, 20);
    affinity_manager.update_worker_metadata(worker3_metadata).await;

    info!("\n📊 Worker Characteristics:");
    info!("  Worker 1: Fast SSD, us-west, load=20%, has blob_abc123");
    info!("  Worker 2: GPU/ML, us-east, load=70%, has blob_def456");
    info!("  Worker 3: CPU batch, eu-central, load=40%, has blob_ghi789");

    // Submit jobs with different affinity strategies
    info!("\n📝 Submitting jobs with affinity rules...\n");

    // 1. Job with data locality affinity
    let job1 = affinity_manager
        .submit_with_affinity(
            JobSpec::new("process_data").payload(serde_json::json!({
                "blob_hash": "blob_abc123",
                "operation": "transform"
            }))?,
            JobAffinity::new(AffinityStrategy::DataLocality {
                blob_hash: "blob_abc123".to_string(),
            }),
        )
        .await?;
    info!("✅ Job 1 (data locality): Should run on worker-1 (has blob_abc123)");

    // 2. Job preferring closest to node3
    let job2 = affinity_manager
        .submit_with_affinity(
            JobSpec::new("process_data").payload(serde_json::json!({
                "data": "network_sensitive"
            }))?,
            JobAffinity::new(AffinityStrategy::ClosestTo(node3)),
        )
        .await?;
    info!("✅ Job 2 (network proximity): Should prefer worker-2 (15ms to node3)");

    // 3. Job requiring GPU tag
    let job3 = affinity_manager
        .submit_with_affinity(
            JobSpec::new("ml_inference").payload(serde_json::json!({
                "model": "deep_learning"
            }))?,
            JobAffinity::new(AffinityStrategy::RequireTags(vec!["gpu".to_string()])).strict(), // Must have GPU
        )
        .await?;
    info!("✅ Job 3 (GPU required): Must run on worker-2 (has GPU tag)");

    // 4. Job preferring least loaded worker
    let job4 = affinity_manager
        .submit_with_affinity(
            JobSpec::new("process_data").payload(serde_json::json!({
                "type": "background_task"
            }))?,
            JobAffinity::new(AffinityStrategy::LeastLoaded),
        )
        .await?;
    info!("✅ Job 4 (load balancing): Should prefer worker-1 (20% load)");

    // 5. Job with geographic affinity
    let job5 = affinity_manager
        .submit_with_affinity(
            JobSpec::new("process_data").payload(serde_json::json!({
                "compliance": "gdpr"
            }))?,
            JobAffinity::new(AffinityStrategy::Geographic {
                region: "eu-central".to_string(),
            }),
        )
        .await?;
    info!("✅ Job 5 (geographic): Should run in eu-central (worker-3)");

    // Create worker pool
    let pool = WorkerPool::with_manager(manager.clone());

    // Register workers
    pool.register_handler("process_data", DataProcessor::new("worker-1".to_string())).await?;
    pool.register_handler("process_data", DataProcessor::new("worker-2".to_string())).await?;
    pool.register_handler("process_data", DataProcessor::new("worker-3".to_string())).await?;
    pool.register_handler("ml_inference", MLWorker {
        worker_id: "worker-2".to_string(),
    })
    .await?;

    // Start workers
    pool.start(3).await?;

    info!("\n⚙️  Processing jobs with affinity rules...\n");

    // Let jobs process
    sleep(Duration::from_secs(2)).await;

    // Check results
    info!("\n📋 Job Execution Results:");

    for (name, job_id) in [
        ("Data locality (blob_abc123)", &job1),
        ("Network proximity (near node3)", &job2),
        ("GPU required", &job3),
        ("Least loaded", &job4),
        ("Geographic (EU)", &job5),
    ] {
        if let Ok(Some(job)) = manager.get_job(job_id).await {
            if let Some(result) = &job.result {
                if let JobResult::Success(output) = result {
                    if let Some(worker) = output.data.get("processed_by") {
                        info!("  {}: Executed by {}", name, worker);
                    }
                }
            }
        }
    }

    // Demonstrate affinity scoring
    info!("\n📊 Affinity Scoring Demo:");
    let test_job = Job::from_spec(JobSpec::new("test"));
    let test_affinity = JobAffinity::new(AffinityStrategy::LeastLoaded).with_weight(0.8);

    let worker1_meta = WorkerMetadata {
        id: "worker-1".to_string(),
        node_id: node1,
        tags: vec![],
        region: None,
        load: 0.2,
        local_blobs: vec![],
        latencies: HashMap::new(),
        local_shards: vec![],
    };

    let score = affinity_manager.calculate_affinity_score(&test_job, &worker1_meta, &test_affinity);
    info!("  Worker-1 (load=0.2) affinity score: {:.2}", score);
    info!("  Formula: 0.8 * (1.0 - 0.2) + 0.2 * 0.5 = {:.2}", 0.8 * 0.8 + 0.2 * 0.5);

    // Shutdown
    info!("\n🛑 Shutting down...");
    pool.shutdown().await?;
    info!("✅ Demo complete!");

    Ok(())
}
