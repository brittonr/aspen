//! Demonstration of VM-based job execution with Hyperlight.
//!
//! This example shows how to:
//! 1. Build guest binaries (or use pre-built ones)
//! 2. Execute jobs in isolated micro-VMs
//! 3. Handle results from VM execution
//!
//! Requirements:
//! - Linux with KVM support (check /dev/kvm exists)
//! - Built guest binaries in target/guest-binaries/
//!
//! Usage:
//! ```bash
//! # Build guest binaries first
//! ./scripts/build-guest.sh echo-worker
//! ./scripts/build-guest.sh data-processor
//!
//! # Run the demo
//! cargo run --example vm_job_demo --features vm-executor
//! ```

use anyhow::Result;
use aspen_jobs::{HyperlightWorker, Job, JobId, JobSpec, JobStatus, Worker};
use chrono::Utc;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tracing::{info, warn};

#[cfg(not(feature = "vm-executor"))]
fn main() {
    eprintln!("This example requires the 'vm-executor' feature.");
    eprintln!("Run with: cargo run --example vm_job_demo --features vm-executor");
}

#[cfg(feature = "vm-executor")]
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("VM Job Execution Demo");
    info!("======================");

    // Check for KVM support
    if !Path::new("/dev/kvm").exists() {
        warn!("KVM not available - VM execution may fail");
        warn!("Ensure KVM is enabled in your system");
    }

    // Create the worker
    let worker = HyperlightWorker::new()
        .map_err(|e| anyhow::anyhow!("Failed to create worker: {}", e))?;
    info!("Created HyperlightWorker");

    // Demo 1: Echo Worker
    info!("\n--- Demo 1: Echo Worker ---");
    demo_echo_worker(&worker).await?;

    // Demo 2: Data Processor
    info!("\n--- Demo 2: Data Processor ---");
    demo_data_processor(&worker).await?;

    // Demo 3: Nix Build (if available)
    info!("\n--- Demo 3: Nix Build ---");
    demo_nix_build(&worker).await?;

    info!("\nDemo completed successfully!");
    Ok(())
}

#[cfg(feature = "vm-executor")]
async fn demo_echo_worker(worker: &HyperlightWorker) -> Result<()> {
    info!("Testing echo worker...");

    // Try to load pre-built binary
    let binary_path = "target/guest-binaries/echo-worker";
    let binary = if Path::new(binary_path).exists() {
        info!("Loading pre-built echo-worker binary");
        fs::read(binary_path)?
    } else {
        warn!("No pre-built binary found, using stub");
        // Use a minimal stub for demonstration
        vec![0x7f, 0x45, 0x4c, 0x46] // ELF magic number
    };

    // Create job
    let job = Job {
        id: JobId::new(),
        spec: JobSpec::with_native_binary(binary)
            .payload(json!({
                "message": "Hello from the host!",
                "test": true
            }))
            .unwrap_or_else(|_| {
                let mut spec = JobSpec::new("vm_execute");
                spec.payload = json!({"message": "Hello from the host!"});
                spec
            })
            .timeout(Duration::from_secs(2))
            .priority(aspen_jobs::Priority::Normal)
            .build(),
        status: JobStatus::Running,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        retry_count: 0,
        last_error: None,
        result: None,
        worker_id: Some("demo-worker".to_string()),
        queue_item_id: None,
    };

    // Execute in VM
    info!("Executing echo job in VM...");
    let result = worker.execute(job).await;

    // Handle result
    match result {
        aspen_jobs::JobResult::Success(output) => {
            info!("✓ Echo job succeeded");
            info!("Output: {}", serde_json::to_string_pretty(&output.data)?);
        }
        aspen_jobs::JobResult::Failure(failure) => {
            warn!("✗ Echo job failed: {}", failure.reason);
        }
        aspen_jobs::JobResult::Cancelled => {
            warn!("Job was cancelled");
        }
    }

    Ok(())
}

#[cfg(feature = "vm-executor")]
async fn demo_data_processor(worker: &HyperlightWorker) -> Result<()> {
    info!("Testing data processor...");

    // Try to load pre-built binary
    let binary_path = "target/guest-binaries/data-processor";
    let binary = if Path::new(binary_path).exists() {
        info!("Loading pre-built data-processor binary");
        fs::read(binary_path)?
    } else {
        warn!("No pre-built binary found, using stub");
        vec![0x7f, 0x45, 0x4c, 0x46]
    };

    // Create job with data to process
    let job = Job {
        id: JobId::new(),
        spec: JobSpec::with_native_binary(binary)
            .payload(json!({
                "value": 42,
                "operation": "transform"
            }))
            .unwrap_or_else(|_| {
                let mut spec = JobSpec::new("vm_execute");
                spec.payload = json!({"value": 42});
                spec
            })
            .timeout(Duration::from_secs(5))
            .build(),
        status: JobStatus::Running,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        retry_count: 0,
        last_error: None,
        result: None,
        worker_id: Some("demo-worker".to_string()),
        queue_item_id: None,
    };

    // Execute in VM
    info!("Executing data processing job in VM...");
    let result = worker.execute(job).await;

    // Handle result
    match result {
        aspen_jobs::JobResult::Success(output) => {
            info!("✓ Data processing succeeded");
            info!("Output: {}", serde_json::to_string_pretty(&output.data)?);
        }
        aspen_jobs::JobResult::Failure(failure) => {
            warn!("✗ Data processing failed: {}", failure.reason);
        }
        aspen_jobs::JobResult::Cancelled => {
            warn!("Job was cancelled");
        }
    }

    Ok(())
}

#[cfg(feature = "vm-executor")]
async fn demo_nix_build(worker: &HyperlightWorker) -> Result<()> {
    info!("Testing Nix build and execute...");

    // Check if Nix is available
    let nix_available = std::process::Command::new("nix")
        .arg("--version")
        .output()
        .is_ok();

    if !nix_available {
        warn!("Nix not available, skipping Nix build demo");
        return Ok(());
    }

    // Create job with inline Nix expression
    let nix_expr = r#"
    { pkgs ? import <nixpkgs> {} }:
    pkgs.writeScriptBin "nix-job" ''
        #!/usr/bin/env bash
        echo "Hello from Nix-built job!"
        echo "Timestamp: $(date)"
    ''
    "#;

    let job = Job {
        id: JobId::new(),
        spec: JobSpec::with_nix_expr(nix_expr)
            .timeout(Duration::from_secs(30)) // Longer timeout for build
            .build(),
        status: JobStatus::Running,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        retry_count: 0,
        last_error: None,
        result: None,
        worker_id: Some("demo-worker".to_string()),
        queue_item_id: None,
    };

    // Execute (will build with Nix then run in VM)
    info!("Building with Nix and executing in VM...");
    let result = worker.execute(job).await;

    // Handle result
    match result {
        aspen_jobs::JobResult::Success(output) => {
            info!("✓ Nix job succeeded");
            info!("Output: {}", serde_json::to_string_pretty(&output.data)?);
        }
        aspen_jobs::JobResult::Failure(failure) => {
            warn!("✗ Nix job failed: {}", failure.reason);
        }
        aspen_jobs::JobResult::Cancelled => {
            warn!("Job was cancelled");
        }
    }

    Ok(())
}