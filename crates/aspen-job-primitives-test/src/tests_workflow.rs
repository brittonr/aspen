//! Workflow engine multi-step test.
//!
//! Scenario 1: Workflow A → B (on AllSuccess) or A → C (on AnyFailed).
//!             A succeeds → B runs, C does not.
//! Scenario 2: A fails → C runs, B does not.

use std::sync::Arc;

use aspen_jobs::JobManager;
use aspen_jobs::JobResult;
use aspen_jobs::JobSpec;
use aspen_jobs::TransitionCondition;
use aspen_jobs::WorkflowBuilder;
use aspen_jobs::WorkflowManager;
use aspen_jobs::WorkflowStep;
use aspen_jobs::WorkflowTransition;

use crate::TestResult;
use crate::make_store;

pub async fn run() -> TestResult {
    test_workflow_follows_success_path().await?;
    test_workflow_follows_failure_path().await?;
    Ok(())
}

fn build_conditional_workflow() -> aspen_jobs::WorkflowDefinition {
    WorkflowBuilder::new("conditional-test", "step_a")
        .add_step(WorkflowStep {
            name: "step_a".to_string(),
            jobs: vec![JobSpec::new("task-a")],
            transitions: vec![
                WorkflowTransition {
                    condition: TransitionCondition::AllSuccess,
                    target: "step_b".to_string(),
                },
                WorkflowTransition {
                    condition: TransitionCondition::AnyFailed,
                    target: "step_c".to_string(),
                },
            ],
            parallel: false,
            timeout: None,
            retry_on_failure: false,
        })
        .add_step(WorkflowStep {
            name: "step_b".to_string(),
            jobs: vec![JobSpec::new("task-b")],
            transitions: vec![WorkflowTransition {
                condition: TransitionCondition::Always,
                target: "done".to_string(),
            }],
            parallel: false,
            timeout: None,
            retry_on_failure: false,
        })
        .add_step(WorkflowStep {
            name: "step_c".to_string(),
            jobs: vec![JobSpec::new("task-c")],
            transitions: vec![WorkflowTransition {
                condition: TransitionCondition::Always,
                target: "error_done".to_string(),
            }],
            parallel: false,
            timeout: None,
            retry_on_failure: false,
        })
        .add_terminal("done")
        .add_terminal("error_done")
        .build()
}

/// A succeeds → workflow transitions to step_b.
async fn test_workflow_follows_success_path() -> TestResult {
    let store = make_store();
    let manager = Arc::new(JobManager::new(store.clone()));
    manager.initialize().await.map_err(|e| format!("initialize: {e}"))?;

    let wm = WorkflowManager::new(manager.clone(), store.clone());
    let definition = build_conditional_workflow();

    let workflow_id = wm
        .start_workflow(&definition, serde_json::json!({}))
        .await
        .map_err(|e| format!("start_workflow: {e}"))?;

    // Get state — should be in step_a with jobs submitted
    let state = wm.get_workflow_state(&workflow_id).await.map_err(|e| format!("get_workflow_state: {e}"))?;

    if state.state != "step_a" {
        return Err(format!("expected state 'step_a', got '{}'", state.state));
    }

    if state.active_jobs.is_empty() {
        return Err("expected active jobs in step_a".to_string());
    }

    // Complete the job in step_a (mark as success)
    let job_id = state.active_jobs.iter().next().unwrap().clone();
    // mark_started is required before mark_completed
    let _token = manager
        .mark_started(&job_id, "test-worker".to_string())
        .await
        .map_err(|e| format!("mark_started: {e}"))?;
    manager
        .mark_completed(&job_id, JobResult::success(serde_json::json!("done")))
        .await
        .map_err(|e| format!("mark_completed: {e}"))?;

    // Process completion — should trigger AllSuccess → step_b
    wm.process_job_completion(&workflow_id, &job_id, &definition)
        .await
        .map_err(|e| format!("process_job_completion: {e}"))?;

    let state = wm.get_workflow_state(&workflow_id).await.map_err(|e| format!("get_workflow_state after: {e}"))?;

    // Should have transitioned to step_b (not step_c)
    if state.state != "step_b" {
        return Err(format!("expected transition to 'step_b', got '{}'", state.state));
    }

    // Verify history shows A → B transition
    let transitions: Vec<_> = state.history.iter().map(|t| format!("{}→{}", t.from, t.to)).collect();
    println!("  workflow success path: transitions={transitions:?}");

    if !transitions.iter().any(|t| t == "step_a→step_b") {
        return Err(format!("expected step_a→step_b in history, got {transitions:?}"));
    }

    Ok(())
}

/// A fails → workflow transitions to step_c.
async fn test_workflow_follows_failure_path() -> TestResult {
    let store = make_store();
    let manager = Arc::new(JobManager::new(store.clone()));
    manager.initialize().await.map_err(|e| format!("initialize: {e}"))?;

    let wm = WorkflowManager::new(manager.clone(), store.clone());
    let definition = build_conditional_workflow();

    let workflow_id = wm
        .start_workflow(&definition, serde_json::json!({}))
        .await
        .map_err(|e| format!("start_workflow: {e}"))?;

    let state = wm.get_workflow_state(&workflow_id).await.map_err(|e| format!("get_workflow_state: {e}"))?;

    let job_id = state.active_jobs.iter().next().unwrap().clone();

    // mark_started is required before mark_completed
    let _token = manager
        .mark_started(&job_id, "test-worker".to_string())
        .await
        .map_err(|e| format!("mark_started: {e}"))?;
    // Mark the job as failed
    manager
        .mark_completed(&job_id, JobResult::failure("intentional failure"))
        .await
        .map_err(|e| format!("mark_completed (fail): {e}"))?;

    // Process completion — should trigger AnyFailed → step_c
    wm.process_job_completion(&workflow_id, &job_id, &definition)
        .await
        .map_err(|e| format!("process_job_completion: {e}"))?;

    let state = wm.get_workflow_state(&workflow_id).await.map_err(|e| format!("get_workflow_state after: {e}"))?;

    if state.state != "step_c" {
        return Err(format!("expected transition to 'step_c', got '{}'", state.state));
    }

    let transitions: Vec<_> = state.history.iter().map(|t| format!("{}→{}", t.from, t.to)).collect();
    println!("  workflow failure path: transitions={transitions:?}");

    if !transitions.iter().any(|t| t == "step_a→step_c") {
        return Err(format!("expected step_a→step_c in history, got {transitions:?}"));
    }

    Ok(())
}
