//! Example demonstrating CAS-based job workflows.
//!
//! This example shows how workflows use CAS operations to ensure
//! atomic state transitions even with concurrent job completions.

use std::sync::Arc;
use std::time::Duration;

use aspen_core::inmemory::DeterministicKeyValueStore;
use aspen_jobs::Job;
use aspen_jobs::JobManager;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::TransitionCondition;
use aspen_jobs::Worker;
use aspen_jobs::WorkerPool;
use aspen_jobs::WorkflowBuilder;
use aspen_jobs::WorkflowManager;
use aspen_jobs::WorkflowStep;
use aspen_jobs::WorkflowTransition;
use async_trait::async_trait;
use tokio::time::sleep;
use tracing::Level;
use tracing::info;

/// Data validation worker.
struct ValidationWorker;

#[async_trait]
impl Worker for ValidationWorker {
    async fn execute(&self, job: Job) -> JobResult {
        info!("✅ Validating data for job {}", job.id);

        // Simulate validation
        sleep(Duration::from_millis(100)).await;

        // Check if data is valid
        if let Some(valid) = job.spec.payload.get("valid") {
            if valid.as_bool() == Some(false) {
                return JobResult::failure("Validation failed: invalid data");
            }
        }

        JobResult::success(serde_json::json!({
            "validated": true,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["validate".to_string()]
    }
}

/// Data transformation worker.
struct TransformWorker;

#[async_trait]
impl Worker for TransformWorker {
    async fn execute(&self, job: Job) -> JobResult {
        info!("🔄 Transforming data for job {}", job.id);

        // Simulate transformation
        sleep(Duration::from_millis(200)).await;

        JobResult::success(serde_json::json!({
            "transformed": true,
            "records": 1000,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["transform".to_string()]
    }
}

/// Data enrichment worker.
struct EnrichmentWorker;

#[async_trait]
impl Worker for EnrichmentWorker {
    async fn execute(&self, job: Job) -> JobResult {
        info!("✨ Enriching data for job {}", job.id);

        // Simulate enrichment
        sleep(Duration::from_millis(150)).await;

        JobResult::success(serde_json::json!({
            "enriched": true,
            "fields_added": 5,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["enrich".to_string()]
    }
}

/// Notification worker.
struct NotificationWorker;

#[async_trait]
impl Worker for NotificationWorker {
    async fn execute(&self, job: Job) -> JobResult {
        info!("📧 Sending notification for job {}", job.id);

        // Get notification type
        let notification_type = job.spec.payload.get("type").and_then(|v| v.as_str()).unwrap_or("info");

        // Simulate sending notification
        sleep(Duration::from_millis(50)).await;

        info!("  Notification sent: {}", notification_type);

        JobResult::success(serde_json::json!({
            "notified": true,
            "type": notification_type,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    fn job_types(&self) -> Vec<String> {
        vec!["notify".to_string()]
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 Starting CAS Workflow Demo\n");

    // Create store and managers
    let store = Arc::new(DeterministicKeyValueStore::new());
    let manager = Arc::new(JobManager::new(store.clone()));
    let workflow_manager = Arc::new(WorkflowManager::new(manager.clone(), store.clone()));

    // Initialize job system
    manager.initialize().await?;

    // Define a data processing pipeline workflow
    info!("📋 Defining data processing workflow...");

    let workflow = WorkflowBuilder::new("data_pipeline", "validation")
        .add_step(WorkflowStep {
            name: "validation".to_string(),
            jobs: vec![JobSpec::new("validate").payload(serde_json::json!({ "valid": true }))?],
            transitions: vec![
                WorkflowTransition {
                    condition: TransitionCondition::AllSuccess,
                    target: "processing".to_string(),
                },
                WorkflowTransition {
                    condition: TransitionCondition::AnyFailed,
                    target: "validation_failed".to_string(),
                },
            ],
            parallel: false,
            timeout: Some(Duration::from_secs(10)),
            retry_on_failure: true,
        })
        .add_step(WorkflowStep {
            name: "processing".to_string(),
            jobs: vec![
                JobSpec::new("transform").payload(serde_json::json!({ "source": "raw_data" }))?,
                JobSpec::new("enrich").payload(serde_json::json!({ "dataset": "customer" }))?,
            ],
            transitions: vec![
                WorkflowTransition {
                    condition: TransitionCondition::AllSuccess,
                    target: "success".to_string(),
                },
                WorkflowTransition {
                    condition: TransitionCondition::SuccessRate(0.5),
                    target: "partial_success".to_string(),
                },
                WorkflowTransition {
                    condition: TransitionCondition::AnyFailed,
                    target: "processing_failed".to_string(),
                },
            ],
            parallel: true, // Transform and enrich can run in parallel
            timeout: Some(Duration::from_secs(30)),
            retry_on_failure: false,
        })
        .add_step(WorkflowStep {
            name: "success".to_string(),
            jobs: vec![
                JobSpec::new("notify")
                    .payload(serde_json::json!({ "type": "success", "message": "Pipeline completed successfully" }))?,
            ],
            transitions: vec![], // Terminal state
            parallel: false,
            timeout: None,
            retry_on_failure: false,
        })
        .add_step(WorkflowStep {
            name: "partial_success".to_string(),
            jobs: vec![
                JobSpec::new("notify")
                    .payload(serde_json::json!({ "type": "warning", "message": "Pipeline completed with warnings" }))?,
            ],
            transitions: vec![], // Terminal state
            parallel: false,
            timeout: None,
            retry_on_failure: false,
        })
        .add_step(WorkflowStep {
            name: "validation_failed".to_string(),
            jobs: vec![
                JobSpec::new("notify")
                    .payload(serde_json::json!({ "type": "error", "message": "Validation failed" }))?,
            ],
            transitions: vec![], // Terminal state
            parallel: false,
            timeout: None,
            retry_on_failure: false,
        })
        .add_step(WorkflowStep {
            name: "processing_failed".to_string(),
            jobs: vec![
                JobSpec::new("notify")
                    .payload(serde_json::json!({ "type": "error", "message": "Processing failed" }))?,
            ],
            transitions: vec![], // Terminal state
            parallel: false,
            timeout: None,
            retry_on_failure: false,
        })
        .add_terminal("success")
        .add_terminal("partial_success")
        .add_terminal("validation_failed")
        .add_terminal("processing_failed")
        .timeout(Duration::from_secs(60))
        .build();

    info!("  Workflow has {} steps", workflow.steps.len());
    info!("  Initial state: {}", workflow.initial_state);
    info!("  Terminal states: {:?}", workflow.terminal_states);

    // Create worker pool
    let pool = WorkerPool::with_manager(manager.clone());

    // Register workers
    pool.register_handler("validate", ValidationWorker).await?;
    pool.register_handler("transform", TransformWorker).await?;
    pool.register_handler("enrich", EnrichmentWorker).await?;
    pool.register_handler("notify", NotificationWorker).await?;

    // Start workers
    pool.start(4).await?;

    info!("\n🎬 Starting workflow instance...");

    // Start workflow
    let workflow_id = workflow_manager
        .start_workflow(
            &workflow,
            serde_json::json!({
                "input_file": "data.csv",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        )
        .await?;

    info!("  Workflow ID: {}", workflow_id);

    // Monitor workflow progress
    info!("\n📊 Monitoring workflow progress...");

    for i in 0..20 {
        sleep(Duration::from_millis(500)).await;

        let state = workflow_manager.get_workflow_state(&workflow_id).await?;

        info!(
            "  [{:2}s] State: {} | Active: {} | Completed: {} | Failed: {}",
            i / 2,
            state.state,
            state.active_jobs.len(),
            state.completed_jobs.len(),
            state.failed_jobs.len()
        );

        // Check if workflow reached terminal state
        if workflow.terminal_states.contains(&state.state) {
            info!("\n✅ Workflow reached terminal state: {}", state.state);
            break;
        }

        // Simulate job completions (in real system, workers would trigger this)
        for job_id in state.active_jobs.clone() {
            // Check if job should be marked complete
            if let Ok(Some(job)) = manager.get_job(&job_id).await {
                if job.status == aspen_jobs::JobStatus::Completed {
                    workflow_manager.process_job_completion(&workflow_id, &job_id, &workflow).await?;
                }
            }
        }
    }

    // Get final workflow state
    let final_state = workflow_manager.get_workflow_state(&workflow_id).await?;

    info!("\n📈 Final Workflow State:");
    info!("  State: {}", final_state.state);
    info!("  Version: {} (CAS updates)", final_state.version);
    info!("  Completed jobs: {}", final_state.completed_jobs.len());
    info!("  Failed jobs: {}", final_state.failed_jobs.len());
    info!("  State transitions: {}", final_state.history.len());

    if !final_state.history.is_empty() {
        info!("\n🔄 State Transition History:");
        for transition in &final_state.history {
            info!("  {} → {} at {}", transition.from, transition.to, transition.timestamp.format("%H:%M:%S"));
        }
    }

    // Demonstrate concurrent CAS updates
    info!("\n🔀 Testing concurrent CAS updates...");

    let workflow_id2 = workflow_manager.start_workflow(&workflow, serde_json::json!({ "test": "concurrent" })).await?;

    // Spawn multiple tasks updating the same workflow concurrently
    let mut handles = vec![];
    for i in 0..5 {
        let wf_manager = Arc::clone(&workflow_manager);
        let wf_id = workflow_id2.clone();
        let handle = tokio::spawn(async move {
            wf_manager
                .update_workflow_state(&wf_id, |mut state| {
                    state.data["update"] = serde_json::json!(i);
                    info!("  Thread {} updating workflow (version {})", i, state.version);
                    Ok(state)
                })
                .await
        });
        handles.push(handle);
    }

    // Wait for all updates
    for handle in handles {
        handle.await??;
    }

    let concurrent_state = workflow_manager.get_workflow_state(&workflow_id2).await?;
    info!("  Final version after concurrent updates: {}", concurrent_state.version);
    info!("  CAS ensured atomic updates despite concurrency");

    // Shutdown
    info!("\n🛑 Shutting down...");
    pool.shutdown().await?;
    info!("✅ Demo complete!");

    Ok(())
}
