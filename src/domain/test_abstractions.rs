//! Simple test to verify domain abstractions compile and work

#[cfg(test)]
mod tests {
    use crate::domain::validation::{JobIdValidator, JobValidator, ValidationError};
    use crate::domain::types::{Job, JobStatus, WorkerType};
    use crate::work_state_machine::WorkStateMachine;

    #[test]
    fn test_validation_works() {
        let job = Job {
            id: "test-123".to_string(),
            status: JobStatus::Pending,
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
            created_at: 1000,
            updated_at: 1000,
            started_at: None,
            error_message: None,
            retry_count: 0,
            payload: serde_json::json!({"test": "data"}),
            compatible_worker_types: vec![],
        };

        let validator = JobIdValidator::new();
        assert!(validator.validate(&job).is_ok(), "Valid job ID should pass");

        let invalid_job = Job {
            id: "".to_string(),
            ..job
        };
        assert!(validator.validate(&invalid_job).is_err(), "Empty ID should fail");
    }

    #[test]
    fn test_state_machine_works() {
        // Valid transition
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::Pending,
            &JobStatus::Claimed
        ).is_ok());

        // Invalid transition
        assert!(WorkStateMachine::validate_transition(
            &JobStatus::Completed,
            &JobStatus::Pending
        ).is_err());
    }

    #[test]
    fn test_worker_type_compatibility() {
        use crate::domain::compatibility::is_compatible_with_worker_type;

        let job = Job {
            id: "test".to_string(),
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

        // Unconstrained job is compatible with any worker
        assert!(is_compatible_with_worker_type(&job, Some(WorkerType::Firecracker)));
        assert!(is_compatible_with_worker_type(&job, Some(WorkerType::Wasm)));

        let constrained_job = Job {
            compatible_worker_types: vec![WorkerType::Firecracker],
            ..job
        };

        // Constrained job only compatible with specified type
        assert!(is_compatible_with_worker_type(&constrained_job, Some(WorkerType::Firecracker)));
        assert!(!is_compatible_with_worker_type(&constrained_job, Some(WorkerType::Wasm)));
    }
}