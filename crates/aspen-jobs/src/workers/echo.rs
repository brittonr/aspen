//! Echo worker for testing - processes any job type by echoing payload.

use async_trait::async_trait;
use tracing::info;

use crate::Job;
use crate::JobResult;
use crate::Worker;

/// Job types that require specialized workers and should NOT be handled by EchoWorker.
/// These jobs should wait in the queue until their specialized handlers register.
const EXCLUDED_JOB_TYPES: &[&str] = &[
    "ci_vm",        // Requires VM workers to boot and register
    "ci_nix_build", // Requires NixBuildWorker
];

/// Fallback worker that echoes job payload as result.
///
/// Handles any job type not handled by specific workers. This is useful for
/// testing job submission and execution without requiring specialized handlers.
///
/// Note: Certain job types (ci_vm, ci_nix_build) are excluded from echo handling
/// because they require specialized workers that may need time to register.
/// These jobs will remain in the queue until their specialized handlers join.
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
        // Empty = handles any job type as fallback (but see can_handle for exclusions)
        vec![]
    }

    fn can_handle(&self, job_type: &str) -> bool {
        // Exclude job types that require specialized workers
        !EXCLUDED_JOB_TYPES.contains(&job_type)
    }

    fn excluded_types(&self) -> Vec<String> {
        // Return the list of excluded types so they can be filtered during dequeue
        EXCLUDED_JOB_TYPES.iter().map(|s| (*s).to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_worker_excludes_ci_vm() {
        let worker = EchoWorker;

        // ci_vm should be excluded
        assert!(!worker.can_handle("ci_vm"));

        // ci_nix_build should be excluded
        assert!(!worker.can_handle("ci_nix_build"));

        // Other job types should be handled
        assert!(worker.can_handle("shell_command"));
        assert!(worker.can_handle("test_job"));
        assert!(worker.can_handle("arbitrary_type"));
    }

    #[test]
    fn test_echo_worker_job_types_empty() {
        let worker = EchoWorker;

        // job_types() returns empty (wildcard), but can_handle() excludes certain types
        assert!(worker.job_types().is_empty());
    }
}
