//! Test that the new domain abstractions work correctly

use mvm_ci::domain::{
    validation::{create_default_validator, JobValidator},
    compatibility::{create_default_checker, is_compatible_with_worker_type},
    types::{Job, JobStatus, Worker, WorkerType, WorkerStatus},
};

fn main() {
    println!("Testing domain abstractions...\n");

    // Test 1: Job Validation
    println!("=== Testing Job Validation ===");
    let validator = create_default_validator();

    let valid_job = Job {
        id: "test-job-123".to_string(),
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

    match validator.validate_job(&valid_job) {
        Ok(()) => println!("✓ Valid job passed validation"),
        Err(errors) => println!("✗ Valid job failed: {:?}", errors),
    }

    let invalid_job = Job {
        id: "".to_string(), // Invalid: empty ID
        created_at: 0, // Invalid: zero timestamp
        ..valid_job.clone()
    };

    match validator.validate_job(&invalid_job) {
        Ok(()) => println!("✗ Invalid job should have failed"),
        Err(errors) => println!("✓ Invalid job correctly failed with {} errors", errors.len()),
    }

    // Test 2: Compatibility Checking
    println!("\n=== Testing Compatibility Checking ===");

    let checker = create_default_checker();

    let worker = Worker {
        id: "worker-1".to_string(),
        worker_type: WorkerType::Firecracker,
        status: WorkerStatus::Online,
        endpoint_id: "endpoint-1".to_string(),
        registered_at: 1000,
        last_heartbeat: 1000,
        cpu_cores: Some(4),
        memory_mb: Some(8192),
        active_jobs: 0,
        total_jobs_completed: 0,
        metadata: serde_json::json!({}),
    };

    // Job with no constraints should be compatible
    if checker.check_compatibility(&valid_job, &worker) {
        println!("✓ Unconstrained job is compatible with worker");
    } else {
        println!("✗ Unconstrained job should be compatible");
    }

    // Job with matching constraint should be compatible
    let mut constrained_job = valid_job.clone();
    constrained_job.compatible_worker_types = vec![WorkerType::Firecracker];

    if checker.check_compatibility(&constrained_job, &worker) {
        println!("✓ Job with matching constraint is compatible");
    } else {
        println!("✗ Job with matching constraint should be compatible");
    }

    // Job with non-matching constraint should not be compatible
    constrained_job.compatible_worker_types = vec![WorkerType::Wasm];

    if !checker.check_compatibility(&constrained_job, &worker) {
        println!("✓ Job with non-matching constraint is not compatible");
    } else {
        println!("✗ Job with non-matching constraint should not be compatible");
    }

    // Test 3: Helper function
    println!("\n=== Testing Helper Functions ===");

    if is_compatible_with_worker_type(&valid_job, Some(WorkerType::Firecracker)) {
        println!("✓ Helper function works for unconstrained job");
    } else {
        println!("✗ Helper function failed");
    }

    println!("\n=== All tests completed ===");
}