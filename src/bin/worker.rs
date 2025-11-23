// Generic Worker Binary
//
// A worker that connects to the control plane via iroh+h3 and executes jobs
// using a pluggable backend (Flawless WASM by default).
// Cache bust: 1732360124

use anyhow::Result;
use mvm_ci::{WorkQueueClient, WorkStatus, WorkerBackend, worker_flawless::FlawlessWorker};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    tracing::info!("Starting mvm-ci worker");

    // Get control plane ticket from environment
    let control_plane_ticket = std::env::var("CONTROL_PLANE_TICKET")
        .expect("CONTROL_PLANE_TICKET environment variable must be set");

    // Get Flawless server URL (defaults to localhost)
    let flawless_url = std::env::var("FLAWLESS_URL")
        .unwrap_or_else(|_| "http://localhost:27288".to_string());

    // Connect to control plane
    tracing::info!("Connecting to control plane via iroh+h3");
    let client = WorkQueueClient::connect(&control_plane_ticket).await?;
    tracing::info!(node_id = %client.node_id(), "Connected to control plane");

    // Initialize worker backend (Flawless WASM)
    tracing::info!("Initializing Flawless worker backend");
    let worker = FlawlessWorker::new(&flawless_url).await?;
    worker.initialize().await?;

    tracing::info!("Worker initialized - starting job processing loop");

    // Main worker loop
    loop {
        tracing::info!("Loop iteration starting - about to call claim_work()");

        match client.claim_work().await {
            Ok(Some(job)) => {
                tracing::info!(
                    job_id = %job.job_id,
                    status = ?job.status,
                    "Claimed job from control plane"
                );

                // Mark as in-progress
                if let Err(e) = client.update_status(&job.job_id, WorkStatus::InProgress).await {
                    tracing::error!(job_id = %job.job_id, error = %e, "Failed to mark job as in-progress");
                }

                // Execute the job using the worker backend
                match worker.execute(job.clone()).await {
                    Ok(result) if result.success => {
                        tracing::info!(job_id = %job.job_id, "Job completed successfully");
                        if let Err(e) = client.update_status(&job.job_id, WorkStatus::Completed).await {
                            tracing::error!(job_id = %job.job_id, error = %e, "Failed to mark job as completed");
                        }
                    }
                    Ok(result) => {
                        tracing::error!(
                            job_id = %job.job_id,
                            error = ?result.error,
                            "Job failed"
                        );
                        if let Err(e) = client.update_status(&job.job_id, WorkStatus::Failed).await {
                            tracing::error!(job_id = %job.job_id, error = %e, "Failed to mark job as failed");
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            job_id = %job.job_id,
                            error = %e,
                            "Job execution error"
                        );
                        if let Err(e) = client.update_status(&job.job_id, WorkStatus::Failed).await {
                            tracing::error!(job_id = %job.job_id, error = %e, "Failed to mark job as failed");
                        }
                    }
                }
            }
            Ok(None) => {
                // No work available, wait before polling again
                tracing::info!("No work available - sleeping for 2 seconds");
                tokio::time::sleep(Duration::from_secs(2)).await;
                tracing::info!("Sleep completed - looping again");
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to claim work from control plane");
                tracing::info!("Error occurred - sleeping for 5 seconds");
                tokio::time::sleep(Duration::from_secs(5)).await;
                tracing::info!("Error sleep completed - looping again");
            }
        }
    }
}
