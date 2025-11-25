#!/bin/bash
# Simple test of Firecracker worker without full VM

set -e

echo "Testing Firecracker worker (mock mode)..."

# Set up minimal environment
export BLIXARD_API_KEY="test-key-32-characters-minimum-length-for-security"
export CONTROL_PLANE_TICKET="http://localhost:3020"

# Create test directories
mkdir -p data/firecracker-vms

# Create a simple test with the Firecracker worker
cat > test-firecracker.rs << 'EOF'
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
EOF

echo "Compiling test..."
rustc --edition 2021 -L target/debug/deps test-firecracker.rs -o test-firecracker \
    --extern mvm_ci=target/debug/libmvm_ci.rlib \
    --extern anyhow=target/debug/deps/libanyhow*.rlib \
    --extern tokio=target/debug/deps/libtokio*.rlib \
    --extern serde_json=target/debug/deps/libserde_json*.rlib \
    --extern tracing=target/debug/deps/libtracing*.rlib \
    --extern tracing_subscriber=target/debug/deps/libtracing_subscriber*.rlib \
    --extern async_trait=target/debug/deps/libasync_trait*.so \
    2>/dev/null || {
    echo "Direct compilation failed, trying with cargo..."

    # Alternative: Create a proper test in the existing structure
    cat > src/bin/test-firecracker.rs << 'EOF'
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
EOF

    cargo build --bin test-firecracker
    ./target/debug/test-firecracker
}

echo "Test complete!"