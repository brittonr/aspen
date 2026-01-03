//! Echo worker for testing - processes any job type by echoing payload.

use async_trait::async_trait;
use tracing::info;

use crate::{Job, JobResult, Worker};

/// Fallback worker that echoes job payload as result.
///
/// Handles any job type not handled by specific workers. This is useful for
/// testing job submission and execution without requiring specialized handlers.
pub struct EchoWorker;

#[async_trait]
impl Worker for EchoWorker {
    async fn execute(&self, job: Job) -> JobResult {
        info!(
            job_id = %job.id,
            job_type = %job.spec.job_type,
            "echo worker processing job"
        );

        // Echo back the payload as the result
        JobResult::success(job.spec.payload.clone())
    }

    fn job_types(&self) -> Vec<String> {
        // Empty = handles any job type as fallback
        vec![]
    }
}
