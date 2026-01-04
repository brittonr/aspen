//! Example demonstrating the job queue system.

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

/// Example email worker.
struct EmailWorker;

#[async_trait]
impl Worker for EmailWorker {
    async fn execute(&self, job: Job) -> JobResult {
        info!("📧 Sending email - Job ID: {}", job.id);

        // Extract email data from payload
        let to = job.spec.payload["to"].as_str().unwrap_or("unknown");
        let subject = job.spec.payload["subject"].as_str().unwrap_or("No subject");

        info!("  To: {}", to);
        info!("  Subject: {}", subject);

        // Simulate email sending
        sleep(Duration::from_millis(500)).await;

        JobResult::success(serde_json::json!({
            "message": format!("Email sent to {}", to),
            "timestamp": Utc::now().to_rfc3339()
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["send_email".to_string()]
    }
}

/// Example data processing worker.
struct DataProcessor;

#[async_trait]
impl Worker for DataProcessor {
    async fn execute(&self, job: Job) -> JobResult {
        info!("🔄 Processing data - Job ID: {}", job.id);

        let data_size = job.spec.payload["size"].as_u64().unwrap_or(100);

        // Simulate data processing
        for i in 0..5 {
            info!("  Processing batch {}/5 ({} items)", i + 1, data_size);
            sleep(Duration::from_millis(200)).await;
        }

        JobResult::success(serde_json::json!({
            "processed_items": data_size * 5,
            "duration_ms": 1000
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["process_data".to_string()]
    }
}

/// Example report generator that can fail.
struct ReportGenerator;

#[async_trait]
impl Worker for ReportGenerator {
    async fn execute(&self, job: Job) -> JobResult {
        info!("📊 Generating report - Job ID: {}", job.id);

        let report_type = job.spec.payload["type"].as_str().unwrap_or("daily");

        // Simulate random failures for demonstration
        if job.attempts == 1 && report_type == "weekly" {
            info!("  ❌ Simulating failure (will retry)");
            return JobResult::failure("Database connection failed");
        }

        info!("  Report type: {}", report_type);
        sleep(Duration::from_millis(800)).await;

        JobResult::success(serde_json::json!({
            "report_url": format!("https://reports.example.com/{}.pdf", report_type),
            "pages": 42
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

    info!("🚀 Starting Job Queue Demo");

    // Create in-memory store for demo
    let store = Arc::new(DeterministicKeyValueStore::new());

    // Create job manager
    let manager = JobManager::new(store.clone());
    manager.initialize().await?;
    info!("✅ Job manager initialized");

    // Submit various jobs
    info!("\n📝 Submitting jobs...");

    // 1. High priority email
    let email_job = manager
        .submit(
            JobSpec::new("send_email")
                .payload(serde_json::json!({
                    "to": "ceo@company.com",
                    "subject": "Urgent: Quarterly Report Ready",
                    "body": "Please review the attached report."
                }))?
                .priority(Priority::High)
                .tag("email")
                .tag("urgent"),
        )
        .await?;
    info!("  ✉️  Email job submitted: {}", email_job);

    // 2. Normal priority data processing
    let data_job = manager
        .submit(
            JobSpec::new("process_data")
                .payload(serde_json::json!({
                    "size": 1000,
                    "type": "customer_analytics"
                }))?
                .priority(Priority::Normal)
                .retry_policy(RetryPolicy::exponential(3)),
        )
        .await?;
    info!("  📊 Data job submitted: {}", data_job);

    // 3. Low priority report with retry
    let report_job = manager
        .submit(
            JobSpec::new("generate_report")
                .payload(serde_json::json!({
                    "type": "weekly",
                    "format": "pdf"
                }))?
                .priority(Priority::Low)
                .retry_policy(RetryPolicy::fixed(3, Duration::from_secs(2))),
        )
        .await?;
    info!("  📈 Report job submitted: {}", report_job);

    // 4. Scheduled job for the future
    let scheduled_job = manager
        .submit(
            JobSpec::new("send_email")
                .payload(serde_json::json!({
                    "to": "team@company.com",
                    "subject": "Daily Standup Reminder"
                }))?
                .schedule_after(Duration::from_secs(5))
                .tag("scheduled"),
        )
        .await?;
    info!("  ⏰ Scheduled job submitted: {} (in 5 seconds)", scheduled_job);

    // 5. Critical priority job
    let critical_job = manager
        .submit(
            JobSpec::new("process_data")
                .payload(serde_json::json!({
                    "size": 10,
                    "type": "security_scan"
                }))?
                .priority(Priority::Critical)
                .timeout(Duration::from_secs(10)),
        )
        .await?;
    info!("  🚨 Critical job submitted: {}", critical_job);

    // Check queue statistics
    let stats = manager.get_queue_stats().await?;
    info!("\n📊 Queue Statistics:");
    info!("  Total queued: {}", stats.total_queued);
    info!("  Processing: {}", stats.processing);
    if let Some(critical_count) = stats.by_priority.get(&Priority::Critical) {
        info!("  Critical priority: {}", critical_count);
    }
    if let Some(high_count) = stats.by_priority.get(&Priority::High) {
        info!("  High priority: {}", high_count);
    }
    if let Some(normal_count) = stats.by_priority.get(&Priority::Normal) {
        info!("  Normal priority: {}", normal_count);
    }
    if let Some(low_count) = stats.by_priority.get(&Priority::Low) {
        info!("  Low priority: {}", low_count);
    }

    // Create worker pool
    info!("\n👷 Starting worker pool...");
    let pool = WorkerPool::new(store.clone());

    // Register workers
    pool.register_handler("send_email", EmailWorker).await?;
    pool.register_handler("process_data", DataProcessor).await?;
    pool.register_handler("generate_report", ReportGenerator).await?;

    // Start workers (note: in real implementation, these would process jobs from queue)
    pool.start(3).await?;
    info!("  Workers started: 3");

    // Simulate job processing
    info!("\n⚙️  Simulating job execution...");

    // Mark jobs as started and completed for demo
    manager.mark_started(&email_job, "worker-1".to_string()).await?;
    sleep(Duration::from_millis(500)).await;
    manager.mark_completed(&email_job, JobResult::success("Email sent")).await?;
    info!("  ✅ Email job completed");

    manager.mark_started(&critical_job, "worker-2".to_string()).await?;
    manager.update_progress(&critical_job, 50, Some("Scanning...".to_string())).await?;
    sleep(Duration::from_millis(300)).await;
    manager.update_progress(&critical_job, 100, Some("Scan complete".to_string())).await?;
    manager.mark_completed(&critical_job, JobResult::success("No threats found")).await?;
    info!("  ✅ Critical security scan completed");

    manager.mark_started(&data_job, "worker-3".to_string()).await?;
    sleep(Duration::from_millis(1000)).await;
    manager.mark_completed(&data_job, JobResult::success("Processed 5000 items")).await?;
    info!("  ✅ Data processing completed");

    // Check job status
    info!("\n📋 Job Status Check:");
    for job_id in &[&email_job, &data_job, &report_job, &scheduled_job, &critical_job] {
        let status = manager.get_status(job_id).await?;
        info!("  Job {}: {:?}", job_id, status);
    }

    // Process scheduled jobs
    info!("\n⏱️  Waiting for scheduled job...");
    sleep(Duration::from_secs(5)).await;

    let processed = manager.process_scheduled().await?;
    info!("  Processed {} scheduled jobs", processed);

    // Cancel a job
    info!("\n❌ Cancelling report job...");
    manager.cancel_job(&report_job).await?;
    info!("  Report job cancelled");

    // Final statistics
    let final_stats = manager.get_queue_stats().await?;
    info!("\n📊 Final Queue Statistics:");
    info!("  Total queued: {}", final_stats.total_queued);
    info!("  Processing: {}", final_stats.processing);

    // Get worker pool stats
    let worker_stats = pool.get_stats().await;
    info!("\n👷 Worker Pool Statistics:");
    info!("  Total workers: {}", worker_stats.total_workers);
    info!("  Idle workers: {}", worker_stats.idle_workers);
    info!("  Processing: {}", worker_stats.processing_workers);

    // Shutdown
    info!("\n🛑 Shutting down...");
    pool.shutdown().await?;
    info!("✅ Demo complete!");

    Ok(())
}
