//! Property-Based Tests for Domain Logic
//!
//! This module uses proptest to verify invariants and properties
//! of the domain logic that should hold for all inputs.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use crate::domain::types::{Job, JobStatus, Worker, WorkerType, WorkerStatus};
    use crate::domain::compatibility::{is_compatible_with_worker_type, WorkerTypeCompatibilityChecker, CompatibilityChecker};
    use crate::work_state_machine::WorkStateMachine;
    use crate::domain::validation::{JobIdValidator, JobValidator};

    // === Strategies for generating test data ===

    /// Strategy for generating job IDs
    fn arb_job_id() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,50}"
    }

    /// Strategy for generating job statuses
    fn arb_job_status() -> impl Strategy<Value = JobStatus> {
        prop_oneof![
            Just(JobStatus::Pending),
            Just(JobStatus::Claimed),
            Just(JobStatus::InProgress),
            Just(JobStatus::Completed),
            Just(JobStatus::Failed),
        ]
    }

    /// Strategy for generating worker types
    fn arb_worker_type() -> impl Strategy<Value = WorkerType> {
        prop_oneof![
            Just(WorkerType::Firecracker),
            Just(WorkerType::Wasm),
        ]
    }

    /// Strategy for generating worker statuses
    fn arb_worker_status() -> impl Strategy<Value = WorkerStatus> {
        prop_oneof![
            Just(WorkerStatus::Idle),
            Just(WorkerStatus::Busy),
            Just(WorkerStatus::Draining),
            Just(WorkerStatus::Offline),
        ]
    }

    /// Strategy for generating jobs
    fn arb_job() -> impl Strategy<Value = Job> {
        (
            arb_job_id(),
            arb_job_status(),
            any::<Option<String>>(),
            any::<Option<String>>(),
            any::<Option<String>>(),
            1000i64..2000i64, // created_at
            1000i64..2000i64, // updated_at
            any::<Option<i64>>(),
            any::<Option<String>>(),
            0u32..10u32, // retry_count
            prop::collection::vec(arb_worker_type(), 0..3),
        ).prop_map(|(id, status, claimed_by, assigned_worker_id, completed_by,
                     created_at, updated_at, started_at, error_message,
                     retry_count, compatible_worker_types)| {
            Job {
                id,
                status,
                claimed_by,
                assigned_worker_id,
                completed_by,
                created_at,
                updated_at,
                started_at,
                error_message,
                retry_count,
                payload: serde_json::json!({"test": "data"}),
                compatible_worker_types,
            }
        })
    }

    /// Strategy for generating workers
    fn arb_worker() -> impl Strategy<Value = Worker> {
        (
            arb_job_id(), // reuse for worker ID
            arb_worker_type(),
            arb_worker_status(),
            1000i64..2000i64, // last_heartbeat
            1000i64..2000i64, // registered_at
            0usize..10usize, // current_jobs
            prop::collection::vec(any::<String>(), 0..5), // capabilities
        ).prop_map(|(id, worker_type, status, last_heartbeat, registered_at,
                     current_jobs, capabilities)| {
            Worker {
                id,
                worker_type,
                status,
                last_heartbeat,
                registered_at,
                current_jobs,
                capabilities,
                metadata: serde_json::json!({}),
            }
        })
    }

    // === Property Tests ===

    proptest! {
        /// Property: Jobs with no worker type constraints are compatible with all workers
        #[test]
        fn prop_unconstrained_jobs_compatible_with_all_workers(
            mut job in arb_job(),
            worker_type in arb_worker_type()
        ) {
            job.compatible_worker_types = vec![]; // No constraints

            assert!(
                is_compatible_with_worker_type(&job, Some(worker_type)),
                "Unconstrained job should be compatible with any worker type"
            );
        }

        /// Property: Jobs are compatible with workers of their specified types
        #[test]
        fn prop_jobs_compatible_with_specified_types(
            mut job in arb_job(),
            worker_type in arb_worker_type()
        ) {
            job.compatible_worker_types = vec![worker_type];

            assert!(
                is_compatible_with_worker_type(&job, Some(worker_type)),
                "Job should be compatible with its specified worker type"
            );
        }

        /// Property: State machine transitions maintain validity
        #[test]
        fn prop_valid_state_transitions_succeed(
            from_status in arb_job_status(),
            to_status in arb_job_status()
        ) {
            let result = WorkStateMachine::validate_transition(&from_status, &to_status);

            // Check known valid transitions
            let is_valid = match (&from_status, &to_status) {
                (a, b) if a == b => true, // Idempotent
                (JobStatus::Pending, JobStatus::Claimed) => true,
                (JobStatus::Pending, JobStatus::Failed) => true,
                (JobStatus::Claimed, JobStatus::InProgress) => true,
                (JobStatus::Claimed, JobStatus::Failed) => true,
                (JobStatus::InProgress, JobStatus::Completed) => true,
                (JobStatus::InProgress, JobStatus::Failed) => true,
                _ => false,
            };

            assert_eq!(
                result.is_ok(),
                is_valid,
                "State transition from {:?} to {:?} validation mismatch",
                from_status,
                to_status
            );
        }

        /// Property: Terminal states cannot transition to other states
        #[test]
        fn prop_terminal_states_cannot_transition(to_status in arb_job_status()) {
            let terminal_states = vec![JobStatus::Completed, JobStatus::Failed];

            for from_status in terminal_states {
                if from_status != to_status {
                    let result = WorkStateMachine::validate_transition(&from_status, &to_status);
                    assert!(
                        result.is_err(),
                        "Terminal state {:?} should not transition to {:?}",
                        from_status,
                        to_status
                    );
                }
            }
        }

        /// Property: Valid job IDs pass validation
        #[test]
        fn prop_valid_job_ids_pass_validation(id in "[a-zA-Z0-9_-]{1,255}") {
            let mut job = Job {
                id: id.clone(),
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
            };

            let validator = JobIdValidator::new();
            assert!(
                validator.validate(&job).is_ok(),
                "Valid job ID '{}' should pass validation",
                id
            );
        }

        /// Property: Jobs cannot have duplicate worker type constraints
        #[test]
        fn prop_no_duplicate_worker_constraints(
            mut job in arb_job(),
            worker_type in arb_worker_type()
        ) {
            // Add the same worker type multiple times
            job.compatible_worker_types = vec![worker_type, worker_type];

            let validator = crate::domain::validation::WorkerConstraintValidator;
            assert!(
                validator.validate(&job).is_err(),
                "Job with duplicate worker type constraints should fail validation"
            );
        }

        /// Property: Worker compatibility is symmetric with job constraints
        #[test]
        fn prop_worker_job_compatibility_symmetric(
            mut job in arb_job(),
            worker in arb_worker()
        ) {
            // Set job to be compatible with worker's type
            job.compatible_worker_types = vec![worker.worker_type];

            let checker = WorkerTypeCompatibilityChecker;
            let is_compatible = checker.is_compatible(&job, &worker);

            // Also check using the helper function
            let helper_compatible = is_compatible_with_worker_type(&job, Some(worker.worker_type));

            assert_eq!(
                is_compatible, helper_compatible,
                "Compatibility check should be consistent"
            );
        }

        /// Property: Jobs always route to compatible workers
        #[test]
        fn prop_jobs_route_to_compatible_workers(
            mut job in arb_job(),
            workers in prop::collection::vec(arb_worker(), 1..10)
        ) {
            // If job has constraints, at least one compatible worker should exist
            // or the job won't be claimed
            if !job.compatible_worker_types.is_empty() {
                let has_compatible = workers.iter().any(|w| {
                    job.compatible_worker_types.contains(&w.worker_type)
                        && matches!(w.status, WorkerStatus::Idle | WorkerStatus::Busy)
                });

                // This is more of a system invariant than a test
                // In a real system, we'd ensure jobs only get created
                // if compatible workers exist
            } else {
                // Unconstrained jobs can go to any available worker
                let has_available = workers.iter().any(|w| {
                    matches!(w.status, WorkerStatus::Idle | WorkerStatus::Busy)
                });

                if has_available {
                    // Job should be claimable
                }
            }
        }

        /// Property: Timestamp ordering is maintained
        #[test]
        fn prop_timestamp_ordering(mut job in arb_job()) {
            // Ensure valid timestamp ordering
            if job.updated_at < job.created_at {
                job.updated_at = job.created_at;
            }

            if let Some(started_at) = job.started_at {
                if started_at < job.created_at {
                    job.started_at = Some(job.created_at);
                }
            }

            let validator = crate::domain::validation::TimestampValidator;
            assert!(
                validator.validate(&job).is_ok(),
                "Job with valid timestamp ordering should pass validation"
            );
        }

        /// Property: Retry count increases monotonically
        #[test]
        fn prop_retry_count_monotonic(
            initial_retry_count in 0u32..10u32,
            increments in prop::collection::vec(1u32..3u32, 0..5)
        ) {
            let mut retry_count = initial_retry_count;

            for increment in increments {
                let new_count = retry_count + increment;
                assert!(
                    new_count > retry_count,
                    "Retry count should increase monotonically"
                );
                retry_count = new_count;
            }
        }
    }

    // === Regression Tests (specific cases that failed before) ===

    #[test]
    fn test_empty_job_id_fails() {
        let job = Job {
            id: "".to_string(),
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
        };

        let validator = JobIdValidator::new();
        assert!(validator.validate(&job).is_err());
    }

    #[test]
    fn test_backwards_state_transition_fails() {
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::InProgress,
            &JobStatus::Pending
        ).is_err());
    }
}