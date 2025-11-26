// Generic Worker Binary
//
// A worker that connects to the control plane via iroh+h3 and executes jobs
// using a pluggable backend (Flawless WASM, Firecracker, etc.).
// Cache bust: 1732360124

use anyhow::{anyhow, Result};
use mvm_ci::{
    AppConfig, WorkQueueClient, JobStatus, WorkerRegistration,
    WorkerType, WorkerHeartbeat,
    state::factory::{InfrastructureFactory, ProductionInfrastructureFactory},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    tracing::info!("Starting mvm-ci worker");

    // Load centralized configuration
    let config = AppConfig::load().expect("Failed to load configuration");

    // Get control plane ticket from environment (worker-specific, not in config)
    let control_plane_ticket = std::env::var("CONTROL_PLANE_TICKET")
        .expect("CONTROL_PLANE_TICKET environment variable must be set");

    // Determine worker type from environment (defaults to WASM)
    let worker_type_str = std::env::var("WORKER_TYPE")
        .unwrap_or_else(|_| "wasm".to_string())
        .to_lowercase();

    let worker_type = match worker_type_str.as_str() {
        "wasm" | "flawless" => WorkerType::Wasm,
        "firecracker" | "vm" => WorkerType::Firecracker,
        _ => {
            return Err(anyhow!("Unknown worker type: {}. Must be 'wasm' or 'firecracker'", worker_type_str));
        }
    };

    // Connect to control plane
    tracing::info!("Connecting to control plane via iroh+h3");
    let client = WorkQueueClient::connect(&control_plane_ticket).await?;
    tracing::info!(node_id = %client.node_id(), "Connected to control plane");

    // Create infrastructure factory for worker creation
    let factory = ProductionInfrastructureFactory::new();

    // For Firecracker workers, we need a VM manager instance
    // Workers can operate in two modes:
    // 1. Standalone mode: Create their own VM manager (current behavior)
    // 2. Shared mode: Use a shared VM manager from infrastructure
    // For now, we'll maintain standalone mode for backward compatibility
    let vm_manager = if worker_type == WorkerType::Firecracker {
        // Create standalone VM manager for Firecracker worker
        use mvm_ci::hiqlite_service::HiqliteService;
        use mvm_ci::vm_manager::{VmManager, VmManagerConfig};

        let worker_state_dir = config.storage.vm_state_dir.join("worker");

        // Create worker-specific Hiqlite instance
        let hiqlite = Arc::new(
            HiqliteService::new(Some(worker_state_dir.join("hiqlite")))
                .await
                .map_err(|e| anyhow!("Failed to create Hiqlite service: {}", e))?
        );

        // Initialize schema
        hiqlite.initialize_schema().await
            .map_err(|e| anyhow!("Failed to initialize Hiqlite schema: {}", e))?;

        // Create VM Manager configuration
        let vm_config = VmManagerConfig {
            max_vms: config.vm.max_concurrent_vms,
            auto_scaling: true,
            pre_warm_count: 2,
            flake_dir: config.vm.flake_dir.clone(),
            state_dir: worker_state_dir.clone(),
            default_memory_mb: config.vm.default_memory_mb,
            default_vcpus: config.vm.default_vcpus,
        };

        // Create VM Manager
        let vm_manager = Arc::new(VmManager::new(vm_config, hiqlite).await?);

        // Start VM manager background tasks
        let manager_clone = vm_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = manager_clone.start_monitoring().await {
                tracing::error!(error = ?e, "VM monitoring failed");
            }
        });

        Some(vm_manager)
    } else {
        None
    };

    // Create worker using factory pattern
    tracing::info!("Creating {} worker using factory pattern",
        if worker_type == WorkerType::Wasm { "WASM" } else { "Firecracker" });

    let worker = factory.create_worker(&config, worker_type, vm_manager).await?;

    worker.initialize().await?;

    // Register with control plane
    tracing::info!(worker_type = ?worker_type, "Registering worker with control plane");
    let registration = WorkerRegistration {
        worker_type,
        endpoint_id: client.node_id().to_string(),
        cpu_cores: Some(num_cpus::get() as u32),
        memory_mb: Some(get_available_memory_mb()),
        metadata: serde_json::json!({
            "hostname": hostname::get().ok().and_then(|h| h.into_string().ok()).unwrap_or_else(|| "unknown".to_string()),
            "version": env!("CARGO_PKG_VERSION"),
            "rust_version": env!("CARGO_PKG_RUST_VERSION"),
        }),
    };

    let registered_worker = client.register_worker(registration).await?;
    let worker_id = registered_worker.id.clone();
    tracing::info!(
        worker_id = %worker_id,
        worker_type = ?registered_worker.worker_type,
        "Worker registered successfully"
    );

    // Track active jobs for heartbeat reporting
    let active_jobs_count = Arc::new(AtomicU32::new(0));

    // Spawn heartbeat task
    let heartbeat_client = client.clone();
    let heartbeat_worker_id = worker_id.clone();
    let heartbeat_active_jobs = active_jobs_count.clone();
    let heartbeat_interval = Duration::from_secs(
        std::env::var("WORKER_HEARTBEAT_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30)
    );

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(heartbeat_interval).await;

            let heartbeat = WorkerHeartbeat {
                worker_id: heartbeat_worker_id.clone(),
                active_jobs: heartbeat_active_jobs.load(Ordering::Relaxed),
                cpu_cores: Some(num_cpus::get() as u32),
                memory_mb: Some(get_available_memory_mb()),
            };

            match heartbeat_client.send_heartbeat(&heartbeat_worker_id, heartbeat).await {
                Ok(_) => {
                    tracing::debug!(worker_id = %heartbeat_worker_id, "Heartbeat sent successfully");
                }
                Err(e) => {
                    tracing::error!(worker_id = %heartbeat_worker_id, error = %e, "Failed to send heartbeat");
                }
            }
        }
    });

    tracing::info!("Worker initialized - starting job processing loop");

    // Main worker loop
    loop {
        tracing::info!("Loop iteration starting - about to call claim_work()");

        match client.claim_work(Some(&worker_id), Some(worker_type)).await {
            Ok(Some(job)) => {
                tracing::info!(
                    job_id = %job.id,
                    status = ?job.status,
                    "Claimed job from control plane"
                );

                // Increment active jobs count
                active_jobs_count.fetch_add(1, Ordering::Relaxed);

                // Mark as in-progress
                if let Err(e) = client.update_status(&job.id, JobStatus::InProgress, None).await {
                    tracing::error!(job_id = %job.id, error = %e, "Failed to mark job as in-progress");
                }

                // Execute the job using the worker backend
                match worker.execute(job.clone()).await {
                    Ok(result) if result.success => {
                        tracing::info!(job_id = %job.id, "Job completed successfully");
                        if let Err(e) = client.update_status(&job.id, JobStatus::Completed, None).await {
                            tracing::error!(job_id = %job.id, error = %e, "Failed to mark job as completed");
                        }
                    }
                    Ok(result) => {
                        let error_msg = result.error.clone().unwrap_or_else(|| "Job failed".to_string());
                        tracing::error!(
                            job_id = %job.id,
                            error = %error_msg,
                            "Job failed"
                        );
                        if let Err(e) = client.update_status(&job.id, JobStatus::Failed, Some(error_msg)).await {
                            tracing::error!(job_id = %job.id, error = %e, "Failed to mark job as failed");
                        }
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        tracing::error!(
                            job_id = %job.id,
                            error = %error_msg,
                            "Job execution error"
                        );
                        if let Err(e) = client.update_status(&job.id, JobStatus::Failed, Some(error_msg)).await {
                            tracing::error!(job_id = %job.id, error = %e, "Failed to mark job as failed");
                        }
                    }
                }

                // Decrement active jobs count
                active_jobs_count.fetch_sub(1, Ordering::Relaxed);
            }
            Ok(None) => {
                // No work available, wait before polling again
                let sleep_duration = config.timing.worker_no_work_sleep();
                tracing::info!("No work available - sleeping for {:?}", sleep_duration);
                tokio::time::sleep(sleep_duration).await;
                tracing::info!("Sleep completed - looping again");
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to claim work from control plane");
                let sleep_duration = config.timing.worker_error_sleep();
                tracing::info!("Error occurred - sleeping for {:?}", sleep_duration);
                tokio::time::sleep(sleep_duration).await;
                tracing::info!("Error sleep completed - looping again");
            }
        }
    }
}

/// Get available system memory in megabytes
fn get_available_memory_mb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(kb) = line.split_whitespace().nth(1) {
                        if let Ok(kb_val) = kb.parse::<u64>() {
                            return kb_val / 1024; // Convert KB to MB
                        }
                    }
                }
            }
        }
    }

    // Fallback: estimate based on number of CPUs (rough heuristic: 2GB per core)
    let cores = num_cpus::get() as u64;
    cores * 2048
}
