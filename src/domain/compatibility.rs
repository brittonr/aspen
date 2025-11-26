//! Job-Worker compatibility checking system
//!
//! Provides flexible compatibility checking between jobs and workers,
//! allowing for different matching strategies and extensibility.

use crate::domain::types::{Job, Worker, WorkerStatus, WorkerType};
use std::sync::Arc;

/// Trait for implementing job-worker compatibility checks
pub trait CompatibilityChecker: Send + Sync {
    /// Check if a job is compatible with a worker
    fn is_compatible(&self, job: &Job, worker: &Worker) -> bool;

    /// Name of this checker for debugging
    fn name(&self) -> &str;

    /// Priority of this checker (lower runs first)
    fn priority(&self) -> u32 {
        100
    }

    /// Whether this checker's result is final (stops further checking)
    fn is_authoritative(&self) -> bool {
        false
    }
}

/// Composite compatibility checker that runs multiple checkers
pub struct CompositeCompatibilityChecker {
    checkers: Vec<Arc<dyn CompatibilityChecker>>,
}

impl CompositeCompatibilityChecker {
    pub fn new() -> Self {
        Self {
            checkers: Vec::new(),
        }
    }

    pub fn add_checker(mut self, checker: Arc<dyn CompatibilityChecker>) -> Self {
        self.checkers.push(checker);
        // Sort by priority (lower first)
        self.checkers.sort_by_key(|c| c.priority());
        self
    }

    pub fn is_compatible(&self, job: &Job, worker: &Worker) -> bool {
        for checker in &self.checkers {
            let result = checker.is_compatible(job, worker);

            if checker.is_authoritative() {
                return result;
            }

            if !result {
                return false;
            }
        }

        true
    }
}

/// Checks worker type compatibility
pub struct WorkerTypeCompatibilityChecker;

impl CompatibilityChecker for WorkerTypeCompatibilityChecker {
    fn is_compatible(&self, job: &Job, worker: &Worker) -> bool {
        job.requirements.is_compatible_with(worker.worker_type)
    }

    fn name(&self) -> &str {
        "WorkerTypeCompatibility"
    }

    fn priority(&self) -> u32 {
        10 // Check early - this is a fundamental constraint
    }
}

/// Checks if worker is online and available
pub struct WorkerStatusChecker;

impl CompatibilityChecker for WorkerStatusChecker {
    fn is_compatible(&self, _job: &Job, worker: &Worker) -> bool {
        matches!(worker.status, WorkerStatus::Online)
    }

    fn name(&self) -> &str {
        "WorkerStatus"
    }

    fn priority(&self) -> u32 {
        5 // Check very early - no point checking other things if worker is offline
    }

    fn is_authoritative(&self) -> bool {
        true // If worker is offline, stop checking
    }
}

/// Checks worker capacity
pub struct WorkerCapacityChecker {
    max_concurrent_jobs: usize,
}

impl WorkerCapacityChecker {
    pub fn new(max_concurrent_jobs: usize) -> Self {
        Self { max_concurrent_jobs }
    }
}

impl CompatibilityChecker for WorkerCapacityChecker {
    fn is_compatible(&self, _job: &Job, worker: &Worker) -> bool {
        worker.active_jobs < self.max_concurrent_jobs as u32
    }

    fn name(&self) -> &str {
        "WorkerCapacity"
    }

    fn priority(&self) -> u32 {
        20 // Check after status and type
    }
}

/// Checks custom requirements from job payload
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
                            return true;
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
        Some(wt) => job.requirements.is_compatible_with(wt),
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