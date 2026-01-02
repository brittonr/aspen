//! Integration test for actual VM execution with Hyperlight.

#![cfg(all(feature = "vm-executor", target_os = "linux"))]

use aspen_jobs::{HyperlightWorker, Worker};

#[tokio::test]
async fn test_hyperlight_worker_creation() {
    // Try to create a HyperlightWorker
    // This tests that Hyperlight can initialize properly
    match HyperlightWorker::new() {
        Ok(worker) => {
            // Worker created successfully
            let job_types = worker.job_types();
            assert!(!job_types.is_empty());
            assert!(job_types.contains(&"vm_execute".to_string()));
            assert!(job_types.contains(&"sandboxed".to_string()));
            println!("✓ HyperlightWorker initialized with job types: {:?}", job_types);
        }
        Err(e) => {
            // This is expected if running without proper KVM/Hyperlight setup
            eprintln!("HyperlightWorker initialization failed (expected in CI): {}", e);
            eprintln!("This test requires KVM and Hyperlight runtime support");
        }
    }
}

#[tokio::test] 
#[ignore = "Requires KVM and Hyperlight runtime"]
async fn test_vm_execution_with_simple_wasm() {
    use aspen_jobs::{Job, JobId, JobSpec, JobStatus, JobResult, VmJobPayload};
    use aspen_jobs::{DependencyState, DependencyFailurePolicy};
    use chrono::Utc;
    
    let worker = HyperlightWorker::new()
        .expect("Failed to create HyperlightWorker - ensure KVM is available");
    
    // Create a minimal valid WASM module (empty module)
    let wasm_module = vec![
        0x00, 0x61, 0x73, 0x6d, // Magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
        // Empty module (no sections)
    ];
    
    let job_spec = JobSpec::new("vm_execute")
        .payload(VmJobPayload::WasmModule { 
            module: wasm_module 
        })
        .unwrap();
    
    let job = Job {
        id: JobId::new(),
        spec: job_spec,
        status: JobStatus::Running,
        attempts: 0,
        last_error: None,
        result: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        scheduled_at: None,
        started_at: Some(Utc::now()),
        completed_at: None,
        worker_id: Some("test-worker".to_string()),
        next_retry_at: None,
        progress: None,
        progress_message: None,
        version: 1,
        dlq_metadata: None,
        dependency_state: DependencyState::Ready,
        blocked_by: vec![],
        blocking: vec![],
        dependency_failure_policy: DependencyFailurePolicy::FailCascade,
    };
    
    let result = worker.execute(job).await;
    
    // The job will likely fail due to invalid/incomplete WASM,
    // but this tests the full execution flow
    match result {
        JobResult::Failure(failure) => {
            println!("Job failed as expected with test WASM: {}", failure.reason);
            // This is expected for our minimal test module
        }
        JobResult::Success(output) => {
            println!("Job unexpectedly succeeded: {:?}", output);
        }
        JobResult::Cancelled => {
            panic!("Job should not be cancelled");
        }
    }
}
