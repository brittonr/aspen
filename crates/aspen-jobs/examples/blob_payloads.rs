//! Example demonstrating blob storage for large job payloads.
//!
//! This example shows how large payloads are automatically stored
//! in content-addressed blob storage, with compression and streaming.

use std::sync::Arc;
use std::time::Duration;

use aspen_testing::DeterministicKeyValueStore;
use aspen_jobs::BlobCollection;
use aspen_jobs::BlobHash;
use aspen_jobs::BlobJobManager;
use aspen_jobs::Job;
use aspen_jobs::JobBlobStorage;
use aspen_jobs::JobManager;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::Worker;
use aspen_jobs::WorkerPool;
use async_trait::async_trait;
use tokio::time::sleep;
use tracing::Level;
use tracing::info;

/// Worker that processes large data blobs.
struct DataProcessorWorker;

#[async_trait]
impl Worker for DataProcessorWorker {
    async fn execute(&self, job: Job) -> JobResult {
        info!("📦 Processing job {} with payload", job.id);

        // Check if payload is a blob reference
        if JobBlobStorage::is_blob_reference(&job.spec.payload) {
            info!("  Payload is stored in blob storage (reference detected)");

            if let Some(hash) = job.spec.payload.get("hash") {
                info!("  Blob hash: {}", hash);
            }
            if let Some(size) = job.spec.payload.get("size") {
                info!("  Original size: {} bytes", size);
            }
        } else {
            // Check payload size
            let payload_str = job.spec.payload.to_string();
            info!("  Inline payload size: {} bytes", payload_str.len());

            if let Some(data_type) = job.spec.payload.get("data_type") {
                info!("  Data type: {}", data_type);
            }
        }

        // Simulate processing
        sleep(Duration::from_millis(100)).await;

        // Generate a large result
        let result_data = if job.spec.job_type == "generate_report" {
            // Generate a large report
            let report_lines: Vec<String> =
                (0..10000).map(|i| format!("Report line {}: Processing complete for item #{}", i, i)).collect();

            serde_json::json!({
                "report": report_lines.join("\n"),
                "summary": {
                    "total_lines": 10000,
                    "processed_by": "DataProcessorWorker",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }
            })
        } else {
            serde_json::json!({
                "status": "processed",
                "worker": "DataProcessorWorker",
                "input_size": job.spec.payload.to_string().len()
            })
        };

        JobResult::success(result_data)
    }

    fn job_types(&self) -> Vec<String> {
        vec![
            "process_small".to_string(),
            "process_large".to_string(),
            "generate_report".to_string(),
            "analyze_dataset".to_string(),
        ]
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 Starting Blob Payloads Demo\n");

    // Create store and managers
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = Arc::new(JobManager::new(store.clone()));
    let blob_manager = BlobJobManager::new(manager.clone()).await?;

    // Initialize job system
    manager.initialize().await?;

    // Create worker pool
    let pool = WorkerPool::with_manager(manager.clone());
    pool.register_handler("process_small", DataProcessorWorker).await?;
    pool.register_handler("process_large", DataProcessorWorker).await?;
    pool.register_handler("generate_report", DataProcessorWorker).await?;
    pool.register_handler("analyze_dataset", DataProcessorWorker).await?;
    pool.start(2).await?;

    info!("📝 Submitting jobs with various payload sizes...\n");

    // 1. Small payload (inline storage)
    info!("1. Small Payload (< 1 MB - stored inline):");
    let small_data = serde_json::json!({
        "data_type": "small",
        "content": "This is a small payload that fits inline",
        "items": (0..100).collect::<Vec<_>>(),
    });

    let small_job = JobSpec::new("process_small").payload(small_data)?;

    let small_job_id = blob_manager.submit_with_blob(small_job).await?;
    info!("  Submitted job: {}", small_job_id);

    // 2. Large payload (blob storage)
    info!("\n2. Large Payload (> 1 MB - stored in blob):");
    let large_string = "x".repeat(500_000); // 500 KB of 'x'
    let large_data = serde_json::json!({
        "data_type": "large",
        "dataset": large_string,
        "metadata": {
            "source": "generated",
            "records": 50000,
            "columns": ["id", "name", "value", "timestamp"]
        },
        // Add more data to exceed 1 MB threshold
        "additional_data": vec![0u8; 600_000], // 600 KB more
    });

    let payload_size = serde_json::to_string(&large_data)?.len();
    info!("  Payload size: {} bytes ({:.2} MB)", payload_size, payload_size as f64 / 1_048_576.0);

    let large_job = JobSpec::new("process_large").payload(large_data)?;

    let large_job_id = blob_manager.submit_with_blob(large_job).await?;
    info!("  Submitted job: {} (payload stored in blob)", large_job_id);

    // 3. Multiple large payloads
    info!("\n3. Batch of Large Payloads:");
    let mut batch_job_ids = Vec::new();

    for i in 0..5 {
        let batch_data = serde_json::json!({
            "batch_id": i,
            "large_array": (0..100_000).map(|j| format!("Item {}", j)).collect::<Vec<_>>(), // ~2 MB of strings
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let job = JobSpec::new("analyze_dataset").payload(batch_data)?;

        let job_id = blob_manager.submit_with_blob(job).await?;
        batch_job_ids.push(job_id.clone());
        info!("  Batch job {}: {}", i, job_id);
    }

    // 4. Generate a report with large result
    info!("\n4. Job Generating Large Result:");
    let report_job = JobSpec::new("generate_report").payload(serde_json::json!({
        "report_type": "comprehensive",
        "include_details": true,
    }))?;

    let report_job_id = blob_manager.submit_with_blob(report_job).await?;
    info!("  Report job: {}", report_job_id);

    // Wait for processing
    info!("\n⏳ Processing jobs...");
    sleep(Duration::from_secs(2)).await;

    // Retrieve jobs and check blob storage
    info!("\n📊 Retrieving Job Results:\n");

    // Check small job (should be inline)
    if let Some(job) = manager.get_job(&small_job_id).await? {
        info!("Small Job ({}):", small_job_id);
        let is_blob = JobBlobStorage::is_blob_reference(&job.spec.payload);
        info!("  Payload stored as blob: {}", is_blob);
        if let Some(result) = &job.result {
            info!("  Status: {:?}", result);
        }
    }

    // Check large job (should be blob)
    info!("\nLarge Job ({}):", large_job_id);
    if let Some(job) = manager.get_job(&large_job_id).await? {
        let is_blob = JobBlobStorage::is_blob_reference(&job.spec.payload);
        info!("  Payload stored as blob: {}", is_blob);

        if is_blob {
            // Retrieve actual payload from blob storage
            if let Some(job_with_payload) = blob_manager.get_job_with_blob(&large_job_id).await? {
                let actual_size = job_with_payload.spec.payload.to_string().len();
                info!("  Retrieved payload size: {} bytes", actual_size);

                // Verify data integrity
                if let Some(data_type) = job_with_payload.spec.payload.get("data_type") {
                    info!("  Data type verified: {}", data_type);
                }
            }
        }
    }

    // Check batch jobs
    info!("\nBatch Jobs:");
    for (i, job_id) in batch_job_ids.iter().enumerate() {
        if let Some(job) = manager.get_job(job_id).await? {
            let is_blob = JobBlobStorage::is_blob_reference(&job.spec.payload);
            info!("  Batch {}: {} (blob: {})", i, job_id, is_blob);
        }
    }

    // Store a large result
    info!("\n💾 Storing Large Result in Blob:");
    let large_result = serde_json::json!({
        "analysis_results": vec!["Result"; 200_000], // ~2 MB result
        "statistics": {
            "total": 200000,
            "processed": 200000,
            "errors": 0,
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    blob_manager.store_job_result(report_job_id.as_str(), large_result).await?;
    info!("  Large result stored for job: {}", report_job_id);

    // Get blob statistics
    info!("\n📈 Blob Storage Statistics:");
    let stats = blob_manager.get_blob_stats().await;
    info!("  Total blobs: {}", stats.total_blobs);
    info!("  Total size: {} MB", stats.total_size_bytes as f64 / 1_048_576.0);
    info!("  Average size: {} KB", stats.average_size_bytes as f64 / 1024.0);

    // Demonstrate blob collection
    info!("\n📚 Creating Blob Collection:");
    let mut collection = BlobCollection::new("batch-results".to_string());

    // Add some mock blob hashes
    use aspen_jobs::BlobHash;
    for i in 0..3 {
        let mock_hash = BlobHash::from_hex(&format!("{:016x}", i))?;
        collection.add_blob(mock_hash, 1_000_000 * (i + 1) as u64);
    }

    info!("  Collection: {}", collection.id);
    info!("  Total blobs: {}", collection.blobs.len());
    info!("  Total size: {} MB", collection.total_size / 1_048_576);

    // Show blob storage benefits
    info!("\n✨ Blob Storage Benefits:");
    info!("  • Automatic storage for payloads > 1 MB");
    info!("  • Content-addressed deduplication");
    info!("  • Compression for payloads > 10 KB");
    info!("  • Streaming support for large data");
    info!("  • Reduced memory footprint in job queue");
    info!("  • Zero-copy access to blob data");

    // Shutdown
    info!("\n🛑 Shutting down...");
    pool.shutdown().await?;
    info!("✅ Demo complete!");

    Ok(())
}
