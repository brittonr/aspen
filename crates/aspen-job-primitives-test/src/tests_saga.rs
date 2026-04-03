//! Saga executor test.
//!
//! Scenario 1: 4-step saga, step 3 fails → compensation runs for steps 2, 1 (LIFO).
//! Scenario 2: 4-step saga, all succeed → no compensation.

use aspen_jobs::CompensationResult;
use aspen_jobs::SagaBuilder;
use aspen_jobs::SagaExecutor;
use aspen_jobs::SagaState;

use crate::TestResult;
use crate::make_store;

pub async fn run() -> TestResult {
    test_saga_compensates_on_mid_step_failure().await?;
    test_saga_completes_without_compensation().await?;
    Ok(())
}

/// Check if a step needs compensation (was executed and requires it).
fn step_needs_compensation(step: &aspen_jobs::SagaStep) -> bool {
    step.was_executed && step.requires_compensation && !step.branch_backed
}

/// 4 steps: step 0,1,2 succeed, step 3 fails.
/// Compensation should run for steps 2, 1 in LIFO order (step 0 is the last).
async fn test_saga_compensates_on_mid_step_failure() -> TestResult {
    let store = make_store();
    let executor = SagaExecutor::new(store);

    let definition = SagaBuilder::new("test-saga")
        .description("4-step saga with mid-failure")
        .step("step-0")
        .done()
        .step("step-1")
        .done()
        .step("step-2")
        .done()
        .step("step-3")
        .done()
        .build();

    if definition.step_count() != 4 {
        return Err(format!("expected 4 steps, got {}", definition.step_count()));
    }

    let mut state = executor.start_saga(definition, None).await.map_err(|e| format!("start_saga: {e}"))?;

    // Verify initial state is Executing { current_step: 0 }
    match &state.state {
        SagaState::Executing { current_step: 0 } => {}
        other => return Err(format!("expected Executing(0), got {other:?}")),
    }

    // Execute steps 0, 1, 2 successfully
    for i in 0..3u32 {
        executor
            .complete_step(&mut state, i, Some(format!("output-{i}")))
            .await
            .map_err(|e| format!("complete_step {i}: {e}"))?;
    }

    // State should be Executing { current_step: 3 }
    match &state.state {
        SagaState::Executing { current_step: 3 } => {}
        other => return Err(format!("expected Executing(3), got {other:?}")),
    }

    // Fail step 3
    executor
        .fail_step(&mut state, 3, "intentional failure at step 3".to_string())
        .await
        .map_err(|e| format!("fail_step 3: {e}"))?;

    // Should now be in Compensating state
    match &state.state {
        SagaState::Compensating {
            failed_step,
            current_compensation,
            ..
        } => {
            if *failed_step != 3 {
                return Err(format!("expected failed_step=3, got {failed_step}"));
            }
            // Compensation starts at step 2 (the last successfully executed step)
            if *current_compensation != 2 {
                return Err(format!("expected current_compensation=2, got {current_compensation}"));
            }
        }
        other => return Err(format!("expected Compensating state, got {other:?}")),
    }

    // Run compensation in LIFO order: step 2, step 1, step 0
    let mut compensated_order = Vec::new();

    loop {
        match &state.state {
            SagaState::Compensating {
                current_compensation, ..
            } => {
                let step_idx = *current_compensation;
                let step = state.definition.get_step(step_idx).ok_or(format!("step {step_idx} not found"))?;

                if step_needs_compensation(step) {
                    compensated_order.push(step_idx);
                    executor
                        .complete_compensation(&mut state, step_idx, CompensationResult::Success)
                        .await
                        .map_err(|e| format!("complete_compensation {step_idx}: {e}"))?;
                } else {
                    executor
                        .complete_compensation(&mut state, step_idx, CompensationResult::Skipped)
                        .await
                        .map_err(|e| format!("skip_compensation {step_idx}: {e}"))?;
                }
            }
            SagaState::CompensationCompleted { .. } => break,
            other => return Err(format!("unexpected state during compensation: {other:?}")),
        }
    }

    // Verify LIFO order: 2, 1, 0
    if compensated_order != vec![2, 1, 0] {
        return Err(format!("expected compensation order [2, 1, 0], got {compensated_order:?}"));
    }

    // Should be CompensationCompleted
    match &state.state {
        SagaState::CompensationCompleted { failure_reason, .. } => {
            if !failure_reason.contains("intentional failure") {
                return Err(format!("unexpected failure_reason: {failure_reason}"));
            }
        }
        other => return Err(format!("expected CompensationCompleted, got {other:?}")),
    }

    println!("  saga compensation: LIFO order verified (2→1→0)");
    Ok(())
}

/// All 4 steps succeed → saga completes with no compensation.
async fn test_saga_completes_without_compensation() -> TestResult {
    let store = make_store();
    let executor = SagaExecutor::new(store);

    let definition = SagaBuilder::new("success-saga")
        .step("step-0")
        .done()
        .step("step-1")
        .done()
        .step("step-2")
        .done()
        .step("step-3")
        .done()
        .build();

    let mut state = executor.start_saga(definition, None).await.map_err(|e| format!("start_saga: {e}"))?;

    // Execute all 4 steps successfully
    for i in 0..4u32 {
        executor
            .complete_step(&mut state, i, Some(format!("output-{i}")))
            .await
            .map_err(|e| format!("complete_step {i}: {e}"))?;
    }

    // Should be Completed (not CompensationCompleted)
    match &state.state {
        SagaState::Completed => {}
        other => return Err(format!("expected Completed, got {other:?}")),
    }

    // No compensation results should exist
    for (i, step) in state.definition.steps.iter().enumerate() {
        if step.compensation_result.is_some() {
            return Err(format!("step {i} has compensation result but saga succeeded"));
        }
    }

    println!("  saga success: completed with no compensation");
    Ok(())
}
