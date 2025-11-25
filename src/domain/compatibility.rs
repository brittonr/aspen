//! Worker Compatibility Module
//!
//! This module contains all business logic for determining if a worker can handle a specific job.
//! It provides a trait-based system for extensible compatibility rules.

use crate::domain::types::{Job, Worker, WorkerType};
use std::sync::Arc;

/// Trait for checking job-worker compatibility
///
/// Implement this trait to add custom compatibility logic.
pub trait CompatibilityChecker: Send + Sync {
    /// Check if a worker is compatible with a job
    ///
    /// Returns `true` if the worker can handle the job, `false` otherwise.
    fn is_compatible(&self, job: &Job, worker: &Worker) -> bool;

    /// Get the name of this checker for logging
    fn name(&self) -> &str;

    /// Priority for this checker (higher = checked first)
    /// If any high-priority checker returns false, lower priority checkers are not consulted
    fn priority(&self) -> u32 {
        100
    }

    /// Whether this checker is authoritative
    /// If true and returns false, no other checkers are consulted
    fn is_authoritative(&self) -> bool {
        false
    }
}

/// Composite compatibility checker that runs multiple checkers
pub struct CompositeCompatibilityChecker {
    checkers: Vec<Arc<dyn CompatibilityChecker>>,
}

impl CompositeCompatibilityChecker {
    /// Create a new composite checker
    pub fn new() -> Self {
        Self {
            checkers: Vec::new(),
        }
    }

    /// Add a checker to the composite
    pub fn add_checker(mut self, checker: Arc<dyn CompatibilityChecker>) -> Self {
        self.checkers.push(checker);
        // Sort by priority (highest first)
        self.checkers.sort_by_key(|c| std::cmp::Reverse(c.priority()));
        self
    }

    /// Check if a worker is compatible with a job using all registered checkers
    pub fn check_compatibility(&self, job: &Job, worker: &Worker) -> bool {
        for checker in &self.checkers {
            let is_compatible = checker.is_compatible(job, worker);

            tracing::trace!(
                checker = checker.name(),
                job_id = %job.id,
                worker_id = %worker.id,
                is_compatible = is_compatible,
                "Compatibility check result"
            );

            if !is_compatible && checker.is_authoritative() {
                // Authoritative checker said no - stop checking
                return false;
            }

            if !is_compatible {
                // Non-authoritative checker said no, but continue checking others
                // All checkers must pass for compatibility
                return false;
            }
        }

        // All checkers passed (or no checkers registered)
        true
    }
}

impl Default for CompositeCompatibilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

// === Built-in Compatibility Checkers ===

/// Checker for worker type compatibility
///
/// This is the primary compatibility mechanism - jobs can specify which worker types
/// they are compatible with.
pub struct WorkerTypeCompatibilityChecker;

impl CompatibilityChecker for WorkerTypeCompatibilityChecker {
    fn is_compatible(&self, job: &Job, worker: &Worker) -> bool {
        // If job has no worker type constraints, it's compatible with all workers
        if job.compatible_worker_types.is_empty() {
            return true;
        }

        // Check if worker's type is in the job's compatible types list
        job.compatible_worker_types.contains(&worker.worker_type)
    }

    fn name(&self) -> &str {
        "WorkerTypeCompatibilityChecker"
    }

    fn priority(&self) -> u32 {
        200 // High priority - check type compatibility first
    }

    fn is_authoritative(&self) -> bool {
        true // If types don't match, no point checking other criteria
    }
}

/// Checker for worker status
///
/// Only workers that are not draining can accept new jobs.
pub struct WorkerStatusChecker;

impl CompatibilityChecker for WorkerStatusChecker {
    fn is_compatible(&self, _job: &Job, worker: &Worker) -> bool {
        use crate::domain::types::WorkerStatus;

        matches!(
            worker.status,
            WorkerStatus::Online
        )
    }

    fn name(&self) -> &str {
        "WorkerStatusChecker"
    }

    fn priority(&self) -> u32 {
        190 // Check status early
    }

    fn is_authoritative(&self) -> bool {
        true // If worker is draining/offline, definitely incompatible
    }
}

/// Checker for worker capacity based on active jobs
///
/// Workers can only handle jobs if they have available capacity.
pub struct WorkerCapacityChecker {
    max_concurrent_jobs: u32,
}

impl WorkerCapacityChecker {
    pub fn new(max_concurrent_jobs: u32) -> Self {
        Self {
            max_concurrent_jobs,
        }
    }
}

impl CompatibilityChecker for WorkerCapacityChecker {
    fn is_compatible(&self, _job: &Job, worker: &Worker) -> bool {
        // Use active_jobs field from Worker
        worker.active_jobs < self.max_concurrent_jobs
    }

    fn name(&self) -> &str {
        "WorkerCapacityChecker"
    }

    fn priority(&self) -> u32 {
        180 // Check capacity after type and status
    }

    fn is_authoritative(&self) -> bool {
        true // If at capacity, definitely incompatible
    }
}

/// Checker for custom job requirements based on metadata
///
/// This checker looks at the job payload for specific requirements and matches them
/// against worker metadata.
pub struct CustomRequirementsChecker;

impl CompatibilityChecker for CustomRequirementsChecker {
    fn is_compatible(&self, job: &Job, worker: &Worker) -> bool {
        // Check if job has custom requirements in payload
        if let Some(requirements) = job.payload.get("requirements") {
            if let Some(required_capabilities) = requirements.get("capabilities") {
                if let Some(caps_array) = required_capabilities.as_array() {
                    // Check if worker metadata contains required capabilities
                    if let Some(worker_caps) = worker.metadata.get("capabilities") {
                        if let Some(worker_caps_array) = worker_caps.as_array() {
                            for required_cap in caps_array {
                                if let Some(cap_str) = required_cap.as_str() {
                                    let found = worker_caps_array.iter().any(|c| {
                                        c.as_str().map_or(false, |s| s == cap_str)
                                    });
                                    if !found {
                                        return false;
                                    }
                                }
                            }
                        } else {
                            return false; // No capabilities array in metadata
                        }
                    } else {
                        // No capabilities in metadata, can't match requirements
                        return caps_array.is_empty();
                    }
                }
            }
        }

        true
    }

    fn name(&self) -> &str {
        "CustomRequirementsChecker"
    }

    fn priority(&self) -> u32 {
        100 // Lower priority - check after basic compatibility
    }
}

/// Helper function to check compatibility with optional worker type
///
/// This is used for backward compatibility with code that doesn't have a full Worker object.
pub fn is_compatible_with_worker_type(
    job: &Job,
    worker_type: Option<WorkerType>,
) -> bool {
    match worker_type {
        Some(wt) => {
            // If job has no worker type constraints, it's compatible with all workers
            if job.compatible_worker_types.is_empty() {
                true
            } else {
                // Check if worker type is in the list
                job.compatible_worker_types.contains(&wt)
            }
        }
        // If no worker type specified, allow claiming any job
        None => true,
    }
}

/// Create a default compatibility checker with all built-in checkers
pub fn create_default_checker() -> CompositeCompatibilityChecker {
    CompositeCompatibilityChecker::new()
        .add_checker(Arc::new(WorkerTypeCompatibilityChecker))
        .add_checker(Arc::new(WorkerStatusChecker))
        .add_checker(Arc::new(WorkerCapacityChecker::new(10))) // Default max 10 concurrent jobs
        .add_checker(Arc::new(CustomRequirementsChecker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{JobStatus, WorkerStatus};

    fn create_test_job() -> Job {
        Job {
            id: "test-job".to_string(),
            status: JobStatus::Pending,
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
            created_at: 1000,
            updated_at: 1000,
            started_at: None,
            error_message: None,
            retry_count: 0,
            payload: serde_json::json!({}),
            compatible_worker_types: vec![],
        }
    }

    fn create_test_worker() -> Worker {
        Worker {
            id: "test-worker".to_string(),
            worker_type: WorkerType::Firecracker,
            status: WorkerStatus::Online,
            endpoint_id: "test-endpoint".to_string(),
            registered_at: 1000,
            last_heartbeat: 1000,
            cpu_cores: Some(4),
            memory_mb: Some(8192),
            active_jobs: 0,
            total_jobs_completed: 0,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn test_worker_type_compatibility() {
        let checker = WorkerTypeCompatibilityChecker;

        // No constraints - compatible with any worker
        let job = create_test_job();
        let worker = create_test_worker();
        assert!(checker.is_compatible(&job, &worker));

        // Matching type constraint
        let mut job = create_test_job();
        job.compatible_worker_types = vec![WorkerType::Firecracker];
        assert!(checker.is_compatible(&job, &worker));

        // Non-matching type constraint
        let mut job = create_test_job();
        job.compatible_worker_types = vec![WorkerType::Wasm];
        assert!(!checker.is_compatible(&job, &worker));

        // Multiple constraints with match
        let mut job = create_test_job();
        job.compatible_worker_types = vec![WorkerType::Wasm, WorkerType::Firecracker];
        assert!(checker.is_compatible(&job, &worker));
    }

    #[test]
    fn test_worker_status_checker() {
        let checker = WorkerStatusChecker;
        let job = create_test_job();

        // Online worker - compatible
        let worker = create_test_worker();
        assert!(checker.is_compatible(&job, &worker));

        // Draining worker - not compatible
        let mut worker = create_test_worker();
        worker.status = WorkerStatus::Draining;
        assert!(!checker.is_compatible(&job, &worker));

        // Offline worker - not compatible
        let mut worker = create_test_worker();
        worker.status = WorkerStatus::Offline;
        assert!(!checker.is_compatible(&job, &worker));
    }

    #[test]
    fn test_worker_capacity_checker() {
        let checker = WorkerCapacityChecker::new(2);
        let job = create_test_job();

        // Worker with capacity - compatible
        let worker = create_test_worker();
        assert!(checker.is_compatible(&job, &worker));

        // Worker at capacity - not compatible
        let mut worker = create_test_worker();
        worker.active_jobs = 2;
        assert!(!checker.is_compatible(&job, &worker));

        // Worker over capacity - not compatible
        let mut worker = create_test_worker();
        worker.active_jobs = 3;
        assert!(!checker.is_compatible(&job, &worker));
    }

    #[test]
    fn test_custom_requirements_checker() {
        let checker = CustomRequirementsChecker;

        // No requirements - compatible
        let job = create_test_job();
        let worker = create_test_worker();
        assert!(checker.is_compatible(&job, &worker));

        // Requirements met
        let mut job = create_test_job();
        job.payload = serde_json::json!({
            "requirements": {
                "capabilities": ["gpu", "high-memory"]
            }
        });
        let mut worker = create_test_worker();
        worker.metadata = serde_json::json!({
            "capabilities": ["gpu", "high-memory", "ssd"]
        });
        assert!(checker.is_compatible(&job, &worker));

        // Requirements not met
        let mut job = create_test_job();
        job.payload = serde_json::json!({
            "requirements": {
                "capabilities": ["gpu", "high-memory"]
            }
        });
        let mut worker = create_test_worker();
        worker.metadata = serde_json::json!({
            "capabilities": ["high-memory"] // Missing "gpu"
        });
        assert!(!checker.is_compatible(&job, &worker));
    }

    #[test]
    fn test_composite_checker() {
        let checker = create_default_checker();

        // All conditions met
        let job = create_test_job();
        let worker = create_test_worker();
        assert!(checker.check_compatibility(&job, &worker));

        // Type mismatch - should fail (authoritative)
        let mut job = create_test_job();
        job.compatible_worker_types = vec![WorkerType::Wasm];
        let worker = create_test_worker();
        assert!(!checker.check_compatibility(&job, &worker));

        // Worker draining - should fail (authoritative)
        let job = create_test_job();
        let mut worker = create_test_worker();
        worker.status = WorkerStatus::Draining;
        assert!(!checker.check_compatibility(&job, &worker));
    }

    #[test]
    fn test_is_compatible_with_worker_type_helper() {
        // No constraints
        let job = create_test_job();
        assert!(is_compatible_with_worker_type(&job, Some(WorkerType::Firecracker)));
        assert!(is_compatible_with_worker_type(&job, Some(WorkerType::Wasm)));
        assert!(is_compatible_with_worker_type(&job, None));

        // With constraints
        let mut job = create_test_job();
        job.compatible_worker_types = vec![WorkerType::Firecracker];
        assert!(is_compatible_with_worker_type(&job, Some(WorkerType::Firecracker)));
        assert!(!is_compatible_with_worker_type(&job, Some(WorkerType::Wasm)));
        assert!(is_compatible_with_worker_type(&job, None)); // None always compatible
    }
}