//! Job query handlers.
//!
//! Handles fetching individual jobs and listing jobs with filters.

use std::sync::Arc;

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::JobDetails;
use aspen_client_api::JobGetResultResponse;
use aspen_client_api::JobListResultResponse;
use aspen_core::KeyValueStore;
use aspen_jobs::JobId;
use aspen_jobs::JobManager;
use aspen_jobs::JobResult;
use aspen_jobs::JobStatus;
use aspen_jobs::Priority;
use tracing::debug;
use tracing::warn;

use super::job_status_to_string;

pub(crate) async fn handle_job_get(
    job_manager: &JobManager<dyn KeyValueStore>,
    job_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Getting job: {}", job_id);

    let job_id = JobId::from_string(job_id);

    match job_manager.get_job(&job_id).await {
        Ok(Some(job)) => {
            let details = JobDetails {
                job_id: job.id.to_string(),
                job_type: job.spec.job_type.clone(),
                status: job_status_to_string(&job.status),
                priority: match job.spec.config.priority {
                    Priority::Low => 0,
                    Priority::Normal => 1,
                    Priority::High => 2,
                    Priority::Critical => 3,
                },
                progress: job.progress.unwrap_or(0),
                progress_message: job.progress_message.clone(),
                payload: serde_json::to_string(&job.spec.payload).unwrap_or_default(),
                tags: job.spec.config.tags.to_vec(),
                submitted_at: job.created_at.to_rfc3339(),
                started_at: job.started_at.map(|t| t.to_rfc3339()),
                completed_at: job.completed_at.map(|t| t.to_rfc3339()),
                worker_id: job.worker_id,
                attempts: job.attempts,
                result: job.result.as_ref().and_then(|r| {
                    if let JobResult::Success(output) = r {
                        serde_json::to_string(&output.data).ok()
                    } else {
                        None
                    }
                }),
                error_message: job.result.as_ref().and_then(|r| {
                    if let JobResult::Failure(failure) = r {
                        Some(failure.reason.clone())
                    } else {
                        None
                    }
                }),
            };

            Ok(ClientRpcResponse::JobGetResult(JobGetResultResponse {
                was_found: true,
                job: Some(details),
                error: None,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::JobGetResult(JobGetResultResponse {
            was_found: false,
            job: None,
            error: None,
        })),
        Err(e) => {
            warn!("Failed to get job: {}", e);
            Ok(ClientRpcResponse::JobGetResult(JobGetResultResponse {
                was_found: false,
                job: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

pub(crate) async fn handle_job_list(
    job_manager: &JobManager<dyn KeyValueStore>,
    kv_store: &Arc<dyn KeyValueStore>,
    status: Option<String>,
    job_type: Option<String>,
    tags: Vec<String>,
    limit: Option<u32>,
    _continuation_token: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Listing jobs: status={:?}, type={:?}", status, job_type);

    // Parse status filter
    let status_filter = status.and_then(|s| match s.as_str() {
        "pending" => Some(JobStatus::Pending),
        "scheduled" => Some(JobStatus::Scheduled),
        "running" => Some(JobStatus::Running),
        "completed" => Some(JobStatus::Completed),
        "failed" => Some(JobStatus::Failed),
        "cancelled" => Some(JobStatus::Cancelled),
        _ => None,
    });

    let limit = limit.unwrap_or(100).min(1000) as usize;
    let mut jobs: Vec<JobDetails> = Vec::new();

    // Scan job keys from the store
    let prefix = "__jobs:";

    // Use the KV store scan to find all job keys
    match kv_store
        .scan(aspen_core::ScanRequest {
            prefix: prefix.to_string(),
            limit: Some(limit as u32),
            continuation_token: None,
        })
        .await
    {
        Ok(scan_result) => {
            for entry in scan_result.entries.iter() {
                // Extract job ID from key (format: __jobs:<job_id>)
                if let Some(job_id_str) = entry.key.strip_prefix(prefix) {
                    let job_id = JobId::from_string(job_id_str.to_string());

                    // Fetch the full job details
                    if let Ok(Some(job)) = job_manager.get_job(&job_id).await {
                        // Apply filters
                        if let Some(ref status_filter) = status_filter
                            && job.status != *status_filter
                        {
                            continue;
                        }

                        if let Some(ref type_filter) = job_type
                            && job.spec.job_type != *type_filter
                        {
                            continue;
                        }

                        if !tags.is_empty() {
                            let has_all_tags = tags.iter().all(|tag| job.spec.config.tags.contains(&tag.to_string()));
                            if !has_all_tags {
                                continue;
                            }
                        }

                        // Convert to JobDetails for response
                        let details = JobDetails {
                            job_id: job.id.to_string(),
                            job_type: job.spec.job_type.clone(),
                            status: job_status_to_string(&job.status),
                            priority: match job.spec.config.priority {
                                Priority::Low => 0,
                                Priority::Normal => 1,
                                Priority::High => 2,
                                Priority::Critical => 3,
                            },
                            progress: job.progress.unwrap_or(0),
                            progress_message: job.progress_message.clone(),
                            payload: serde_json::to_string(&job.spec.payload).unwrap_or_default(),
                            tags: job.spec.config.tags.to_vec(),
                            submitted_at: job.created_at.to_rfc3339(),
                            started_at: job.started_at.map(|t| t.to_rfc3339()),
                            completed_at: job.completed_at.map(|t| t.to_rfc3339()),
                            worker_id: job.worker_id,
                            attempts: job.attempts,
                            result: job.result.as_ref().and_then(|r| {
                                if let JobResult::Success(output) = r {
                                    serde_json::to_string(&output.data).ok()
                                } else {
                                    None
                                }
                            }),
                            error_message: job.result.as_ref().and_then(|r| {
                                if let JobResult::Failure(failure) = r {
                                    Some(failure.reason.clone())
                                } else {
                                    None
                                }
                            }),
                        };

                        jobs.push(details);

                        if jobs.len() >= limit {
                            break;
                        }
                    }
                }
            }

            Ok(ClientRpcResponse::JobListResult(JobListResultResponse {
                total_count: jobs.len() as u32,
                jobs,
                continuation_token: scan_result.continuation_token,
                error: None,
            }))
        }
        Err(e) => {
            warn!("Failed to list jobs: {}", e);
            Ok(ClientRpcResponse::JobListResult(JobListResultResponse {
                jobs: vec![],
                total_count: 0,
                continuation_token: None,
                error: Some(e.to_string()),
            }))
        }
    }
}
