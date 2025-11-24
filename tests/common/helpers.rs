//! Test helper functions
//!
//! Provides utility functions for common test operations.

use anyhow::{anyhow, Result};
use mvm_ci::domain::types::{Job, JobStatus, WorkerType};
use mvm_ci::work_queue_client::WorkQueueClient;
use std::time::Duration;

/// Submit a job with worker type constraints
///
/// # Arguments
/// * `client` - WorkQueueClient for submission
/// * `job_id` - Unique job identifier
/// * `worker_types` - List of compatible worker types (empty = any)
/// * `payload` - Job payload data
///
/// # Returns
/// The submitted job ID on success
pub async fn submit_job_with_worker_type(
    _client: &WorkQueueClient,
    job_id: &str,
    _worker_types: Vec<WorkerType>,
    _payload: serde_json::Value,
) -> Result<String> {
    // TODO: Implement job submission endpoint in WorkQueueClient
    // For now, return the job ID
    Ok(job_id.to_string())
}

/// Wait for a job to reach a specific status
///
/// Polls the job status until it matches the expected status or times out.
///
/// # Arguments
/// * `client` - WorkQueueClient for querying
/// * `job_id` - Job to monitor
/// * `expected_status` - Status to wait for
/// * `timeout` - Maximum time to wait
///
/// # Returns
/// The job once it reaches the expected status
///
/// # Errors
/// Returns error if timeout is reached or job not found
pub async fn wait_for_job_status(
    client: &WorkQueueClient,
    job_id: &str,
    expected_status: JobStatus,
    timeout: Duration,
) -> Result<Job> {
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err(anyhow!(
                "Timeout waiting for job {} to reach status {:?}",
                job_id,
                expected_status
            ));
        }

        // Poll for job status by listing all jobs
        let jobs = client.list_work().await?;
        if let Some(job) = jobs.iter().find(|j| j.id == job_id) {
            if job.status == expected_status {
                return Ok(job.clone());
            }
        }

        // Wait before polling again
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Wait for any job to be claimed by a worker
///
/// Polls the job list until a job is claimed or times out.
///
/// # Arguments
/// * `client` - WorkQueueClient for querying
/// * `timeout` - Maximum time to wait
///
/// # Returns
/// The first claimed job found
pub async fn wait_for_any_job_claimed(
    client: &WorkQueueClient,
    timeout: Duration,
) -> Result<Job> {
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err(anyhow!("Timeout waiting for any job to be claimed"));
        }

        // Get all jobs and find first claimed one
        let jobs = client.list_work().await?;
        if let Some(claimed_job) = jobs.into_iter().find(|j| j.status == JobStatus::Claimed) {
            return Ok(claimed_job);
        }

        // Wait before polling again
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Verify a worker is registered in the control plane
///
/// # Arguments
/// * `client` - WorkQueueClient for querying
/// * `worker_id` - Worker ID to check
///
/// # Returns
/// True if worker exists in registry, false otherwise
pub async fn verify_worker_in_registry(
    client: &WorkQueueClient,
    worker_id: &str,
) -> Result<bool> {
    let worker = client.get_worker(worker_id).await?;
    Ok(worker.is_some())
}

/// Create a test job payload
///
/// # Arguments
/// * `url` - URL to include in payload
///
/// # Returns
/// A JSON payload suitable for job submission
pub fn create_test_job_payload(url: &str) -> serde_json::Value {
    serde_json::json!({
        "url": url,
        "test": true,
        "timestamp": chrono::Utc::now().timestamp()
    })
}

/// Generate a unique test job ID
///
/// # Arguments
/// * `prefix` - Prefix for the job ID
///
/// # Returns
/// A unique job ID string
pub fn generate_test_job_id(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4())
}

/// Wait for a condition to become true
///
/// Generic helper for polling arbitrary conditions.
///
/// # Arguments
/// * `condition` - Async function that returns true when condition is met
/// * `timeout` - Maximum time to wait
///
/// # Returns
/// Ok if condition becomes true within timeout
pub async fn wait_for_condition<F, Fut>(
    mut condition: F,
    timeout: Duration,
) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err(anyhow!("Timeout waiting for condition"));
        }

        if condition().await {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_job_payload() {
        let payload = create_test_job_payload("https://example.com");
        assert_eq!(payload["url"], "https://example.com");
        assert_eq!(payload["test"], true);
        assert!(payload["timestamp"].is_number());
    }

    #[test]
    fn test_generate_test_job_id() {
        let id1 = generate_test_job_id("test");
        let id2 = generate_test_job_id("test");

        assert!(id1.starts_with("test-"));
        assert!(id2.starts_with("test-"));
        assert_ne!(id1, id2); // Should be unique
    }
}
