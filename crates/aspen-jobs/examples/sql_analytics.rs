//! Example demonstrating SQL-based job analytics.
//!
//! This example shows how to use SQL queries to analyze job performance,
//! success rates, failure patterns, and generate analytics dashboards.

use aspen_core::inmemory::DeterministicKeyValueStore;
use aspen_jobs::{
    AnalyticsDashboard, AnalyticsQuery, ExportFormat, GroupBy, Job, JobAnalytics, JobManager,
    JobResult, JobSpec, Priority, TimeWindow, Worker, WorkerPool,
};
use async_trait::async_trait;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};

/// Simulated data processing worker.
struct DataWorker {
    worker_id: String,
    failure_rate: f32,
}

#[async_trait]
impl Worker for DataWorker {
    async fn execute(&self, _job: Job) -> JobResult {
        let processing_time = {
            let mut rng = rand::thread_rng();
            rng.gen_range(50..500)
        };

        sleep(Duration::from_millis(processing_time)).await;

        // Simulate failures based on failure rate
        let should_fail = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..1.0) < self.failure_rate
        };

        if should_fail {
            JobResult::failure(format!("Processing failed for worker {}", self.worker_id))
        } else {
            let records_processed = {
                let mut rng = rand::thread_rng();
                rng.gen_range(100..1000)
            };
            JobResult::success(serde_json::json!({
                "worker": self.worker_id,
                "processing_time_ms": processing_time,
                "records_processed": records_processed,
            }))
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec!["process_data".to_string(), "analyze".to_string(), "report".to_string()]
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🚀 Starting SQL Analytics Demo\n");

    // Create store and managers
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = Arc::new(JobManager::new(store.clone()));
    let analytics = JobAnalytics::new(manager.clone(), store.clone());

    // Initialize job system
    manager.initialize().await?;

    // Create worker pool with varying failure rates
    let pool = WorkerPool::with_manager(manager.clone());

    // Register workers with different characteristics
    pool.register_handler("process_data", DataWorker {
        worker_id: "worker-1".to_string(),
        failure_rate: 0.05, // 5% failure rate
    }).await?;

    pool.register_handler("analyze", DataWorker {
        worker_id: "worker-2".to_string(),
        failure_rate: 0.10, // 10% failure rate
    }).await?;

    pool.register_handler("report", DataWorker {
        worker_id: "worker-3".to_string(),
        failure_rate: 0.15, // 15% failure rate
    }).await?;

    // Start workers
    pool.start(3).await?;

    info!("📝 Submitting test jobs for analytics...\n");

    // Submit a mix of jobs with different types and priorities
    let job_types = vec!["process_data", "analyze", "report"];
    let priorities = vec![Priority::Critical, Priority::High, Priority::Normal, Priority::Low];

    for i in 0..100 {
        let job_type = job_types[i % job_types.len()];
        let priority = priorities[i % priorities.len()].clone();

        let job = JobSpec::new(job_type)
            .priority(priority)
            .payload(serde_json::json!({
                "batch_id": i / 10,
                "record_count": rand::thread_rng().gen_range(100..1000),
            }))?;

        manager.submit(job).await?;

        // Small delay to spread jobs over time
        if i % 10 == 0 {
            sleep(Duration::from_millis(100)).await;
        }
    }

    info!("⏳ Waiting for jobs to process...\n");
    sleep(Duration::from_secs(3)).await;

    info!("📊 Running Analytics Queries\n");

    // 1. Success rate analysis
    info!("1. Overall Success Rate:");
    let result = analytics.query(AnalyticsQuery::SuccessRate {
        job_type: None,
        time_window: TimeWindow::Hours(1),
    }).await?;
    display_result(&result);

    // 2. Success rate by job type
    info!("\n2. Success Rate by Job Type:");
    for job_type in &job_types {
        let result = analytics.query(AnalyticsQuery::SuccessRate {
            job_type: Some(job_type.to_string()),
            time_window: TimeWindow::Hours(1),
        }).await?;
        info!("  {}: {:.1}%", job_type,
            result.rows.get(0)
                .and_then(|r| r.get(0))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
        );
    }

    // 3. Average duration analysis
    info!("\n3. Job Duration Statistics:");
    let result = analytics.query(AnalyticsQuery::AverageDuration {
        job_type: None,
        status: Some(aspen_jobs::JobStatus::Completed),
    }).await?;
    display_result(&result);

    // 4. Throughput calculation
    info!("\n4. Job Throughput:");
    let result = analytics.query(AnalyticsQuery::Throughput {
        time_window: TimeWindow::Minutes(5),
    }).await?;
    display_result(&result);

    // 5. Queue depth by priority
    info!("\n5. Current Queue Depth:");
    let result = analytics.query(AnalyticsQuery::QueueDepth {
        priority: None,
    }).await?;
    display_result(&result);

    // 6. Failure analysis by job type
    info!("\n6. Failure Analysis by Job Type:");
    let result = analytics.query(AnalyticsQuery::FailureAnalysis {
        time_window: TimeWindow::Hours(1),
        group_by: GroupBy::JobType,
    }).await?;
    display_result(&result);

    // 7. Failure analysis by error type
    info!("\n7. Top Failure Reasons:");
    let result = analytics.query(AnalyticsQuery::FailureAnalysis {
        time_window: TimeWindow::Hours(1),
        group_by: GroupBy::ErrorType,
    }).await?;
    display_result(&result);

    // Run predefined dashboards
    info!("\n📈 Running Analytics Dashboards\n");

    // Job health dashboard
    info!("Job Health Dashboard:");
    let health_dashboard = AnalyticsDashboard::job_health();
    let health_results = health_dashboard.execute(&analytics).await?;
    for (name, result) in health_results {
        info!("  {}: {} ms execution time", name, result.execution_time_ms);
    }

    // Failure analysis dashboard
    info!("\nFailure Analysis Dashboard:");
    let failure_dashboard = AnalyticsDashboard::failure_analysis();
    let failure_results = failure_dashboard.execute(&analytics).await?;
    for (name, result) in failure_results {
        info!("  {}: {} rows returned", name, result.rows.len());
    }

    // Export analytics data
    info!("\n💾 Exporting Analytics Data\n");

    // Export to JSON
    let json_export = analytics.export(
        AnalyticsQuery::SuccessRate {
            job_type: None,
            time_window: TimeWindow::Hours(1),
        },
        ExportFormat::Json,
    ).await?;
    info!("  JSON export: {} bytes", json_export.len());

    // Export to CSV
    let csv_export = analytics.export(
        AnalyticsQuery::QueueDepth { priority: None },
        ExportFormat::Csv,
    ).await?;
    info!("  CSV export: {} bytes", csv_export.len());
    info!("  CSV preview:\n{}", String::from_utf8_lossy(&csv_export[..csv_export.len().min(200)]));

    // Demonstrate custom SQL query
    info!("\n🔍 Custom SQL Query:");
    let custom_query = AnalyticsQuery::Custom {
        sql: "SELECT job_type, COUNT(*) as count, AVG(duration_ms) as avg_duration
              FROM jobs
              WHERE status = 'Completed'
              GROUP BY job_type
              ORDER BY count DESC".to_string(),
    };
    let result = analytics.query(custom_query).await?;
    info!("  SQL: {}",
        if let AnalyticsQuery::Custom { sql } = &result.query { sql } else { "" });
    info!("  Execution time: {} ms", result.execution_time_ms);

    // Show query performance
    info!("\n⚡ Query Performance Summary:");
    let queries = vec![
        ("Success Rate", AnalyticsQuery::SuccessRate {
            job_type: None,
            time_window: TimeWindow::Hours(1),
        }),
        ("Average Duration", AnalyticsQuery::AverageDuration {
            job_type: None,
            status: None,
        }),
        ("Throughput", AnalyticsQuery::Throughput {
            time_window: TimeWindow::Hours(1),
        }),
        ("Queue Depth", AnalyticsQuery::QueueDepth {
            priority: None,
        }),
    ];

    for (name, query) in queries {
        let start = std::time::Instant::now();
        let _ = analytics.query(query).await?;
        info!("  {}: {:.2} ms", name, start.elapsed().as_secs_f64() * 1000.0);
    }

    // Shutdown
    info!("\n🛑 Shutting down...");
    pool.shutdown().await?;
    info!("✅ Demo complete!");

    Ok(())
}

/// Helper to display analytics results.
fn display_result(result: &aspen_jobs::AnalyticsResult) {
    // Display columns
    info!("  Columns: {}", result.columns.join(", "));

    // Display rows
    for (i, row) in result.rows.iter().take(5).enumerate() {
        let values: Vec<String> = row.iter()
            .map(|v| {
                if let Some(n) = v.as_f64() {
                    format!("{:.2}", n)
                } else {
                    v.to_string()
                }
            })
            .collect();
        info!("  Row {}: {}", i + 1, values.join(", "));
    }

    if result.rows.len() > 5 {
        info!("  ... and {} more rows", result.rows.len() - 5);
    }

    info!("  Execution time: {} ms", result.execution_time_ms);
}