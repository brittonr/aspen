use anyhow::Result;
use mvm_ci::worker_firecracker_v2::{FirecrackerConfig, FirecrackerWorker};
use mvm_ci::{WorkerBackend, Job, JobStatus};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("Initializing Firecracker worker v2...");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let config = FirecrackerConfig {
        flake_dir: std::path::PathBuf::from("./microvms"),
        job_dir: std::path::PathBuf::from(format!("{}/mvm-ci-test/jobs", home)),
        vm_state_dir: std::path::PathBuf::from(format!("{}/mvm-ci-test/vms", home)),
        default_memory_mb: 512,
        default_vcpus: 1,
        control_plane_ticket: "http://localhost:3020".to_string(),
        max_concurrent_vms: 2,
        hypervisor: "qemu".to_string(), // Use QEMU for testing
    };

    let worker = FirecrackerWorker::new(config)?;
    println!("✓ Worker created successfully");

    // Initialize the worker
    match worker.initialize().await {
        Ok(()) => println!("✓ Worker initialized successfully"),
        Err(e) => println!("⚠ Worker initialization warning: {}", e),
    }

    // Create a test job
    let test_job = Job {
        id: "v2-test-001".to_string(),
        status: JobStatus::Pending,
        claimed_by: None,
        assigned_worker_id: None,
        completed_by: None,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        started_at: None,
        error_message: None,
        retry_count: 0,
        payload: serde_json::json!({
            "task": "echo 'Hello from Rust test'",
            "type": "simple",
            "test": true,
            "version": "v2"
        }),
        compatible_worker_types: vec![],
    };

    println!("\nTest job created: {}", test_job.id);
    println!("Configuration:");
    println!("  - Hypervisor: QEMU");
    println!("  - Memory: 512 MB");
    println!("  - vCPUs: 1");
    println!("  - Job directory: ~/mvm-ci-test/jobs");

    // Note: Actual VM execution would happen here with:
    // let result = worker.execute(test_job).await?;

    println!("\n✓ Firecracker worker v2 test completed");
    Ok(())
}
