use anyhow::Result;
use mvm_ci::{FirecrackerConfig, FirecrackerWorker, WorkerBackend, Job, JobStatus};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("Initializing Firecracker worker...");

    let config = FirecrackerConfig {
        flake_dir: std::path::PathBuf::from("./microvms"),
        state_dir: std::path::PathBuf::from("./data/firecracker-vms"),
        default_memory_mb: 512,
        default_vcpus: 1,
        control_plane_ticket: "test-ticket".to_string(),
        max_concurrent_vms: 2,
    };

    let worker = FirecrackerWorker::new(config)?;

    println!("Firecracker worker initialized successfully!");

    // Create a test job
    let test_job = Job {
        id: "test-001".to_string(),
        payload: serde_json::json!({
            "task": "echo 'Hello from test'",
            "type": "simple"
        }),
        priority: 1,
        status: JobStatus::Pending,
        created_at: 0,
        updated_at: 0,
        claimed_by: None,
        assigned_worker_id: None,
        max_retries: 3,
        retry_count: 0,
        scheduled_at: None,
        completed_at: None,
        error_message: None,
        metadata: None,
    };

    println!("Would execute job: {}", test_job.id);
    println!("Configuration validated:");
    println!("  - Flake dir exists: {}", std::path::Path::new("./microvms").exists());
    println!("  - State dir exists: {}", std::path::Path::new("./data/firecracker-vms").exists());
    println!("  - Max concurrent VMs: 2");

    // Note: We can't actually execute the job without a working microvm build
    // println!("Executing test job...");
    // let result = worker.execute(test_job).await?;
    // println!("Job result: {:?}", result);

    Ok(())
}
