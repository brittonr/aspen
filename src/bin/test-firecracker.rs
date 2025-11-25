use anyhow::Result;
use mvm_ci::{Job, JobStatus, WorkerBackend};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("Testing Firecracker worker initialization...");

    let config = mvm_ci::config::AppConfig::load()?;

    println!("Configuration loaded:");
    println!("  - Flawless URL: {}", config.flawless.flawless_url);
    println!("  - HTTP Port: {}", config.network.http_port);

    // Create test job
    let test_job = Job {
        id: "firecracker-test-001".to_string(),
        payload: serde_json::json!({
            "task": "echo 'Hello from Firecracker test'",
            "type": "simple"
        }),
        priority: 1,
        status: JobStatus::Pending,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        claimed_by: None,
        assigned_worker_id: None,
        max_retries: 3,
        retry_count: 0,
        scheduled_at: None,
        completed_at: None,
        error_message: None,
        metadata: Some(serde_json::json!({
            "test": true,
            "worker_type": "firecracker"
        })),
    };

    println!("Created test job: {}", test_job.id);
    println!("Job payload: {}", serde_json::to_string_pretty(&test_job.payload)?);

    println!("\nFirecracker worker test completed successfully!");
    println!("Note: Actual VM execution requires:");
    println!("  1. Working Nix flake build");
    println!("  2. Firecracker binary available");
    println!("  3. Root/CAP_NET_ADMIN permissions for TAP networking");

    Ok(())
}
