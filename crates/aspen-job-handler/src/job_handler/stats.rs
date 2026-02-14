//! Job queue statistics handler.
//!
//! Handles retrieval of job queue statistics including counts by status and priority.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::JobQueueStatsResultResponse;
use aspen_client_api::PriorityCount;
use aspen_core::KeyValueStore;
use aspen_jobs::JobManager;
use aspen_jobs::Priority;
use tracing::debug;
use tracing::warn;

pub(crate) async fn handle_job_queue_stats(
    job_manager: &JobManager<dyn KeyValueStore>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Getting job queue statistics");

    match job_manager.get_queue_stats().await {
        Ok(stats) => {
            // Convert internal stats to response format
            let priority_counts = stats
                .by_priority
                .into_iter()
                .map(|(priority, count)| {
                    let priority_num = match priority {
                        Priority::Low => 0,
                        Priority::Normal => 1,
                        Priority::High => 2,
                        Priority::Critical => 3,
                    };
                    PriorityCount {
                        priority: priority_num,
                        count,
                    }
                })
                .collect();

            Ok(ClientRpcResponse::JobQueueStatsResult(JobQueueStatsResultResponse {
                pending_count: stats.total_queued,
                scheduled_count: 0, // Not available in current QueueStats
                running_count: stats.processing,
                completed_count: 0, // Not available in current QueueStats
                failed_count: 0,    // Not available in current QueueStats
                cancelled_count: 0, // Not available in current QueueStats
                priority_counts,
                type_counts: vec![], // Not available in current QueueStats
                error: None,
            }))
        }
        Err(e) => {
            warn!("Failed to get queue stats: {}", e);
            Ok(ClientRpcResponse::JobQueueStatsResult(JobQueueStatsResultResponse {
                pending_count: 0,
                scheduled_count: 0,
                running_count: 0,
                completed_count: 0,
                failed_count: 0,
                cancelled_count: 0,
                priority_counts: vec![],
                type_counts: vec![],
                error: Some(e.to_string()),
            }))
        }
    }
}
