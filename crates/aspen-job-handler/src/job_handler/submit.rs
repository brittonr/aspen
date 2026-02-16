//! Job submission handler.
//!
//! Handles job submission with priority, retry policy, scheduling, and tags.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::JobSubmitResultResponse;
use aspen_core::KeyValueStore;
use aspen_jobs::JobConfig;
use aspen_jobs::JobManager;
use aspen_jobs::JobSpec;
use aspen_jobs::Priority;
use aspen_jobs::RetryPolicy;
use tracing::debug;
use tracing::info;
use tracing::warn;

#[derive(Debug)]
pub(crate) struct JobSubmitConfig {
    pub(crate) priority: Option<u8>,
    pub(crate) timeout_ms: Option<u64>,
    pub(crate) max_retries: Option<u32>,
    pub(crate) retry_delay_ms: Option<u64>,
    pub(crate) schedule: Option<String>,
    pub(crate) tags: Vec<String>,
}

pub(crate) async fn handle_job_submit(
    job_manager: &JobManager<dyn KeyValueStore>,
    job_type: String,
    payload_str: String,
    config: JobSubmitConfig,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Submitting job: type={}, priority={:?}, schedule={:?}", job_type, config.priority, config.schedule);

    // Parse the JSON payload string
    let payload: serde_json::Value =
        serde_json::from_str(&payload_str).map_err(|e| anyhow::anyhow!("invalid JSON payload: {}", e))?;

    // Parse schedule if provided
    let parsed_schedule = match config.schedule {
        Some(ref schedule_str) => {
            Some(aspen_jobs::parse_schedule(schedule_str).map_err(|e| anyhow::anyhow!("invalid schedule: {}", e))?)
        }
        None => None,
    };

    // Convert priority
    let priority = match config.priority.unwrap_or(1) {
        0 => Priority::Low,
        1 => Priority::Normal,
        2 => Priority::High,
        3 => Priority::Critical,
        _ => Priority::Normal,
    };

    // Create retry policy
    let retry_policy = if let Some(max_attempts) = config.max_retries {
        if max_attempts == 0 {
            RetryPolicy::none()
        } else {
            RetryPolicy::fixed(max_attempts, std::time::Duration::from_millis(config.retry_delay_ms.unwrap_or(1000)))
        }
    } else {
        RetryPolicy::default()
    };

    // Create job config
    let job_config = JobConfig {
        priority,
        retry_policy,
        timeout: config.timeout_ms.map(std::time::Duration::from_millis),
        tags: config.tags.into_iter().collect(),
        dependencies: vec![],
        save_result: true,
        ttl_after_completion: None,
    };

    // Create job spec
    let spec = JobSpec {
        job_type,
        payload,
        config: job_config,
        schedule: parsed_schedule,
        idempotency_key: None,
        metadata: std::collections::HashMap::new(),
    };

    // Submit job
    match job_manager.submit(spec).await {
        Ok(job_id) => {
            info!("Job submitted: {}", job_id);
            Ok(ClientRpcResponse::JobSubmitResult(JobSubmitResultResponse {
                is_success: true,
                job_id: Some(job_id.to_string()),
                error: None,
            }))
        }
        Err(e) => {
            warn!("Failed to submit job: {}", e);
            Ok(ClientRpcResponse::JobSubmitResult(JobSubmitResultResponse {
                is_success: false,
                job_id: None,
                error: Some(e.to_string()),
            }))
        }
    }
}
