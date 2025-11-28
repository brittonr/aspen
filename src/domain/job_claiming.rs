//! Job claiming business logic
//!
//! This module contains the domain logic for claiming jobs from the queue.
//! It handles compatibility checking and job selection, keeping business
//! rules separate from infrastructure concerns.

use crate::domain::types::{Job, JobStatus, WorkerType};

/// Domain service for job claiming logic
///
/// Contains business rules for:
/// - Job compatibility checking
/// - Job selection criteria
/// - Claim validation
///
/// This is pure domain logic, independent of infrastructure.
pub struct JobClaimingService;

impl JobClaimingService {
    /// Check if a job is claimable
    ///
    /// A job is claimable if it's in Pending status.
    pub fn is_claimable(job: &Job) -> bool {
        job.status == JobStatus::Pending
    }

    /// Check if a job is compatible with a worker type
    ///
    /// Business logic for determining if a worker of a given type
    /// can execute a job based on the job's requirements.
    ///
    /// # Arguments
    /// * `job` - The job to check
    /// * `worker_type` - Optional worker type (None means any worker can claim)
    ///
    /// # Returns
    /// true if the job can be claimed by this worker type
    pub fn is_compatible_with_worker_type(job: &Job, worker_type: Option<WorkerType>) -> bool {
        match worker_type {
            Some(wt) => job.requirements.is_compatible_with(wt),
            // If no worker type specified, allow claiming any job
            None => true,
        }
    }

    /// Find the first claimable job from a list that matches worker type
    ///
    /// This encapsulates the business logic for job selection:
    /// - Must be in Pending status
    /// - Must be compatible with worker type (if specified)
    ///
    /// # Arguments
    /// * `jobs` - List of jobs to search
    /// * `worker_type` - Optional worker type filter
    ///
    /// # Returns
    /// The first matching job, or None if no jobs are claimable
    pub fn find_claimable_job(jobs: &[Job], worker_type: Option<WorkerType>) -> Option<Job> {
        jobs.iter()
            .filter(|job| Self::is_claimable(job))
            .find(|job| Self::is_compatible_with_worker_type(job, worker_type))
            .cloned()
    }

    /// Validate that a job can be claimed
    ///
    /// Performs business rule validation before claiming.
    ///
    /// # Arguments
    /// * `job` - The job to validate
    /// * `worker_type` - Optional worker type
    ///
    /// # Returns
    /// Ok if job can be claimed, Err with reason if not
    pub fn validate_claim(job: &Job, worker_type: Option<WorkerType>) -> Result<(), String> {
        if !Self::is_claimable(job) {
            return Err(format!("Job {} is not in Pending status (current: {:?})", job.id, job.status));
        }

        if !Self::is_compatible_with_worker_type(job, worker_type) {
            return Err(format!(
                "Job {} is not compatible with worker type {:?}. Required types: {:?}",
                job.id,
                worker_type,
                job.requirements.compatible_worker_types
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::job_metadata::JobMetadata;
    use crate::domain::job_requirements::JobRequirements;

    #[test]
    fn test_is_claimable_pending_job() {
        let mut job = Job::default();
        job.status = JobStatus::Pending;
        assert!(JobClaimingService::is_claimable(&job));
    }

    #[test]
    fn test_is_not_claimable_claimed_job() {
        let mut job = Job::default();
        job.status = JobStatus::Claimed;
        assert!(!JobClaimingService::is_claimable(&job));
    }

    #[test]
    fn test_compatibility_with_no_requirements() {
        let job = Job {
            requirements: JobRequirements::any(),
            ..Default::default()
        };

        // Any worker type can claim a job with no requirements
        assert!(JobClaimingService::is_compatible_with_worker_type(&job, Some(WorkerType::Ephemeral)));
        assert!(JobClaimingService::is_compatible_with_worker_type(&job, Some(WorkerType::Service)));
        assert!(JobClaimingService::is_compatible_with_worker_type(&job, None));
    }

    #[test]
    fn test_compatibility_with_specific_worker_type() {
        let job = Job {
            requirements: JobRequirements::for_worker_types(vec![WorkerType::Service]),
            ..Default::default()
        };

        // Only Service workers can claim
        assert!(JobClaimingService::is_compatible_with_worker_type(&job, Some(WorkerType::Service)));
        assert!(!JobClaimingService::is_compatible_with_worker_type(&job, Some(WorkerType::Ephemeral)));
        // No worker type specified = allow
        assert!(JobClaimingService::is_compatible_with_worker_type(&job, None));
    }

    #[test]
    fn test_find_claimable_job() {
        let mut jobs = vec![
            Job {
                id: "job1".to_string(),
                status: JobStatus::Completed,
                requirements: JobRequirements::any(),
                ..Default::default()
            },
            Job {
                id: "job2".to_string(),
                status: JobStatus::Pending,
                requirements: JobRequirements::for_worker_types(vec![WorkerType::Service]),
                ..Default::default()
            },
            Job {
                id: "job3".to_string(),
                status: JobStatus::Pending,
                requirements: JobRequirements::any(),
                ..Default::default()
            },
        ];

        // Service worker should claim job2 (first matching)
        let result = JobClaimingService::find_claimable_job(&jobs, Some(WorkerType::Service));
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "job2");

        // Ephemeral worker should skip job2 and claim job3
        let result = JobClaimingService::find_claimable_job(&jobs, Some(WorkerType::Ephemeral));
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "job3");
    }

    #[test]
    fn test_validate_claim_success() {
        let job = Job {
            id: "job1".to_string(),
            status: JobStatus::Pending,
            requirements: JobRequirements::any(),
            ..Default::default()
        };

        assert!(JobClaimingService::validate_claim(&job, None).is_ok());
    }

    #[test]
    fn test_validate_claim_wrong_status() {
        let job = Job {
            id: "job1".to_string(),
            status: JobStatus::Claimed,
            requirements: JobRequirements::any(),
            ..Default::default()
        };

        assert!(JobClaimingService::validate_claim(&job, None).is_err());
    }

    #[test]
    fn test_validate_claim_incompatible_type() {
        let job = Job {
            id: "job1".to_string(),
            status: JobStatus::Pending,
            requirements: JobRequirements::for_worker_types(vec![WorkerType::Service]),
            ..Default::default()
        };

        assert!(JobClaimingService::validate_claim(&job, Some(WorkerType::Ephemeral)).is_err());
    }
}
