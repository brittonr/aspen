//! Integrated example demonstrating full job queue flow with real processing.

use std::sync::Arc;
use std::time::Duration;

use aspen_core::inmemory::DeterministicKeyValueStore;
use aspen_jobs::Job;
use aspen_jobs::JobManager;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::Priority;
use aspen_jobs::RetryPolicy;
use aspen_jobs::Worker;
use aspen_jobs::WorkerPool;
use async_trait::async_trait;
use chrono::Utc;
use tokio::time::sleep;
use tracing::Level;
use tracing::info;
use tracing_subscriber;

/// Email sending worker.
struct EmailWorker;

#[async_trait]
impl Worker for EmailWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let to = job.spec.payload["to"].as_str().unwrap_or("unknown");
        let subject = job.spec.payload["subject"].as_str().unwrap_or("No subject");

        info!("📧 Sending email to {}: {}", to, subject);

        // Simulate email sending
        sleep(Duration::from_millis(200)).await;

        JobResult::success(serde_json::json!({
            "sent_to": to,
            "sent_at": Utc::now().to_rfc3339(),
            "message_id": format!("msg_{}", job.id)
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["email".to_string()]
    }
}

/// Data processing worker that can handle multiple types.
struct DataWorker;

#[async_trait]
impl Worker for DataWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let job_type = &job.spec.job_type;

        match job_type.as_str() {
            "process_batch" => {
                let size = job.spec.payload["size"].as_u64().unwrap_or(100);
                info!("📊 Processing batch of {} items", size);

                // Simulate batch processing
                for i in 1..=3 {
                    info!("  Processing chunk {}/3...", i);
                    sleep(Duration::from_millis(100)).await;
                }

                JobResult::success(serde_json::json!({
                    "processed": size,
                    "chunks": 3,
                    "duration_ms": 300
                }))
            }
            "aggregate_data" => {
                let source = job.spec.payload["source"].as_str().unwrap_or("unknown");
                info!("📈 Aggregating data from source: {}", source);

                sleep(Duration::from_millis(300)).await;

                JobResult::success(serde_json::json!({
                    "source": source,
                    "records_aggregated": 1500,
                    "metrics_generated": 25
                }))
            }
            _ => JobResult::failure(format!("Unknown data job type: {}", job_type)),
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec!["process_batch".to_string(), "aggregate_data".to_string()]
    }
}

/// Report generator that demonstrates retry logic.
struct ReportWorker {
    /// Track attempt for demonstration
    attempt_counter: std::sync::Mutex<u32>,
}

impl ReportWorker {
    fn new() -> Self {
        Self {
            attempt_counter: std::sync::Mutex::new(0),
        }
    }
}

#[async_trait]
impl Worker for ReportWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let report_type = job.spec.payload["type"].as_str().unwrap_or("daily");

        // Simulate failure on first attempt for weekly reports
        {
            let mut counter = self.attempt_counter.lock().unwrap();
            *counter += 1;
        }

        info!("📊 Generating {} report (attempt #{})", report_type, job.attempts);

        if report_type == "weekly" && job.attempts == 1 {
            info!("  ⚠️ Simulating database connection error");
            return JobResult::failure("Database connection timeout");
        }

        // Simulate report generation
        sleep(Duration::from_millis(500)).await;

        JobResult::success(serde_json::json!({
            "report_type": report_type,
            "url": format!("https://reports.example.com/{}/{}.pdf", report_type, job.id),
            "pages": 42,
            "generated_at": Utc::now().to_rfc3339()
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["generate_report".to_string()]
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 Starting Integrated Job Queue Worker Demo");

    // Create store and manager
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = Arc::new(JobManager::new(store.clone()));

    // Initialize job system
    manager.initialize().await?;
    info!("✅ Job system initialized with priority queues");

    // Create worker pool using the same manager
    let pool = WorkerPool::with_manager(manager.clone());

    // Register workers
    pool.register_handler("email", EmailWorker).await?;
    pool.register_handler("process_batch", DataWorker).await?;
    pool.register_handler("aggregate_data", DataWorker).await?;
    pool.register_handler("generate_report", ReportWorker::new()).await?;
    info!("✅ Registered 4 worker handlers");

    // Start workers
    pool.start(3).await?;
    info!("✅ Started 3 workers");

    // Submit various jobs
    info!("\n📝 Submitting jobs to queue...\n");

    // Critical priority email
    let email1 = manager
        .submit(
            JobSpec::new("email")
                .payload(serde_json::json!({
                    "to": "ceo@company.com",
                    "subject": "Q4 Revenue Report",
                    "body": "Please review the attached quarterly report."
                }))?
                .priority(Priority::Critical),
        )
        .await?;
    info!("  Submitted critical email job: {}", email1);

    // High priority batch processing
    let batch1 = manager
        .submit(
            JobSpec::new("process_batch")
                .payload(serde_json::json!({
                    "size": 5000,
                    "type": "customer_transactions"
                }))?
                .priority(Priority::High),
        )
        .await?;
    info!("  Submitted high priority batch job: {}", batch1);

    // Normal priority aggregation
    let agg1 = manager
        .submit(
            JobSpec::new("aggregate_data")
                .payload(serde_json::json!({
                    "source": "sales_database",
                    "period": "last_24h"
                }))?
                .priority(Priority::Normal),
        )
        .await?;
    info!("  Submitted normal priority aggregation: {}", agg1);

    // Low priority report with retry policy
    let report1 = manager
        .submit(
            JobSpec::new("generate_report")
                .payload(serde_json::json!({
                    "type": "weekly",
                    "format": "pdf"
                }))?
                .priority(Priority::Low)
                .retry_policy(RetryPolicy::fixed(3, Duration::from_secs(1))),
        )
        .await?;
    info!("  Submitted low priority report (will retry): {}", report1);

    // More normal priority jobs
    for i in 1..=3 {
        let email = manager
            .submit(
                JobSpec::new("email")
                    .payload(serde_json::json!({
                        "to": format!("user{}@company.com", i),
                        "subject": format!("Newsletter #{}", i)
                    }))?
                    .priority(Priority::Normal),
            )
            .await?;
        info!("  Submitted newsletter email: {}", email);
    }

    // Check initial queue stats
    let stats = manager.get_queue_stats().await?;
    info!("\n📊 Initial Queue Status:");
    info!("  Total queued: {}", stats.total_queued);
    info!("  By priority:");
    for priority in [Priority::Critical, Priority::High, Priority::Normal, Priority::Low] {
        if let Some(count) = stats.by_priority.get(&priority) {
            info!("    {:?}: {}", priority, count);
        }
    }

    // Let workers process jobs
    info!("\n⚙️  Workers processing jobs...\n");

    // Give workers time to process
    sleep(Duration::from_secs(3)).await;

    // Check worker stats
    let worker_stats = pool.get_stats().await;
    info!("\n👷 Worker Statistics:");
    info!("  Jobs processed: {}", worker_stats.total_jobs_processed);
    info!("  Jobs failed: {}", worker_stats.total_jobs_failed);
    info!("  Workers idle: {}", worker_stats.idle_workers);
    info!("  Workers processing: {}", worker_stats.processing_workers);

    // Check final queue stats
    let final_stats = manager.get_queue_stats().await?;
    info!("\n📊 Final Queue Status:");
    info!("  Remaining in queue: {}", final_stats.total_queued);
    info!("  Still processing: {}", final_stats.processing);

    // Check specific job statuses
    info!("\n📋 Job Status Check:");
    for (name, job_id) in [
        ("Critical email", &email1),
        ("Batch processing", &batch1),
        ("Data aggregation", &agg1),
        ("Weekly report", &report1),
    ] {
        let status = manager.get_status(job_id).await?;
        info!("  {}: {:?}", name, status);
    }

    // Schedule a future job
    info!("\n⏰ Scheduling a future job...");
    let scheduled = manager
        .submit(
            JobSpec::new("email")
                .payload(serde_json::json!({
                    "to": "team@company.com",
                    "subject": "Scheduled maintenance notification"
                }))?
                .schedule_after(Duration::from_secs(2)),
        )
        .await?;
    info!("  Scheduled job {} for 2 seconds from now", scheduled);

    // Wait and process scheduled jobs
    sleep(Duration::from_secs(2)).await;
    let processed = manager.process_scheduled().await?;
    info!("  Processed {} scheduled jobs", processed);

    // Give worker time to process the scheduled job
    sleep(Duration::from_millis(500)).await;

    // Final worker stats
    let final_worker_stats = pool.get_stats().await;
    info!("\n✅ Final Results:");
    info!("  Total jobs submitted: 8");
    info!("  Total jobs processed: {}", final_worker_stats.total_jobs_processed);
    info!("  Total jobs failed: {}", final_worker_stats.total_jobs_failed);
    info!(
        "  Success rate: {:.1}%",
        (final_worker_stats.total_jobs_processed as f64
            / (final_worker_stats.total_jobs_processed + final_worker_stats.total_jobs_failed) as f64)
            * 100.0
    );

    // Graceful shutdown
    info!("\n🛑 Shutting down...");
    pool.shutdown().await?;
    info!("✅ Worker pool shut down cleanly");

    Ok(())
}
