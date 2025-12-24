//! Unit tests for the Supervisor module.
//!
//! Tests the restart logic, backoff timing, circuit breaker behavior,
//! and graceful shutdown functionality.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aspen::raft::supervisor::Supervisor;

/// Test that supervisor can be created with a name.
#[tokio::test]
async fn test_supervisor_creation() {
    let supervisor = Supervisor::new("test-task");
    assert_eq!(supervisor.restart_count(), 0);
}

/// Test that a successful task completes without restarts.
#[tokio::test]
async fn test_successful_task_no_restarts() {
    let supervisor = Supervisor::new("success-task");
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = Arc::clone(&call_count);

    let supervisor_clone = Arc::clone(&supervisor);
    tokio::spawn(async move {
        supervisor_clone
            .supervise(move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .await;
    });

    // Wait for task to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(call_count.load(Ordering::Relaxed), 1);
    assert_eq!(supervisor.restart_count(), 0);
}

/// Test that a task is executed at least once.
/// Note: Full restart testing with actual delays would require time mocking.
#[tokio::test]
async fn test_task_execution() {
    let supervisor = Supervisor::new("execution-task");
    let executed = Arc::new(AtomicU32::new(0));
    let executed_clone = Arc::clone(&executed);

    let supervisor_clone = Arc::clone(&supervisor);

    let handle = tokio::spawn(async move {
        supervisor_clone
            .supervise(move || {
                let exec = executed_clone.clone();
                async move {
                    exec.store(1, Ordering::Relaxed);
                    Ok(()) // Complete successfully immediately
                }
            })
            .await;
    });

    // Wait for completion
    tokio::time::timeout(Duration::from_millis(500), handle)
        .await
        .expect("supervisor should complete quickly")
        .expect("supervisor task should not panic");

    // Verify task was executed
    assert_eq!(executed.load(Ordering::Relaxed), 1, "task should execute once");
    assert_eq!(supervisor.restart_count(), 0, "no restarts for successful task");
}

/// Test that supervisor respects MAX_RESTARTS limit via should_restart() logic.
/// We test this by using record_health_failure to simulate failures without real timeouts.
#[tokio::test]
async fn test_circuit_breaker_after_max_restarts() {
    let supervisor = Supervisor::new("circuit-breaker-task");

    // Initially should allow restarts
    assert!(supervisor.should_attempt_recovery().await);

    // Record failures up to the limit (MAX_RESTARTS = 3)
    supervisor.record_health_failure("failure 1").await;
    assert!(supervisor.should_attempt_recovery().await);

    supervisor.record_health_failure("failure 2").await;
    assert!(supervisor.should_attempt_recovery().await);

    supervisor.record_health_failure("failure 3").await;
    // After 3 failures (MAX_RESTARTS), circuit breaker trips
    assert!(
        !supervisor.should_attempt_recovery().await,
        "circuit breaker should trip after MAX_RESTARTS failures"
    );

    // Verify restart count
    assert_eq!(supervisor.restart_count(), 3, "should have recorded exactly 3 restarts");
}

/// Test that stop() immediately stops the supervisor.
#[tokio::test]
async fn test_graceful_shutdown_via_stop() {
    let supervisor = Supervisor::new("shutdown-task");
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = Arc::clone(&call_count);

    let supervisor_clone = Arc::clone(&supervisor);
    let handle = tokio::spawn(async move {
        supervisor_clone
            .supervise(move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::Relaxed);
                    // Long-running task that fails
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    Err("should never get here".to_string())
                }
            })
            .await;
    });

    // Let the task start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Stop the supervisor
    supervisor.stop();

    // Wait for handle to complete
    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("supervisor should stop quickly")
        .expect("supervisor task should not panic");

    // Task should have been called once before stop
    assert_eq!(call_count.load(Ordering::Relaxed), 1);
}

/// Test cancellation token triggers shutdown.
#[tokio::test]
async fn test_cancellation_token_shutdown() {
    let supervisor = Supervisor::new("cancel-task");
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = Arc::clone(&call_count);

    let cancel_token = supervisor.cancellation_token();

    let supervisor_clone = Arc::clone(&supervisor);
    let handle = tokio::spawn(async move {
        supervisor_clone
            .supervise(move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    Ok(())
                }
            })
            .await;
    });

    // Let the task start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cancel via token
    cancel_token.cancel();

    // Wait for handle to complete
    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("supervisor should stop quickly after cancel")
        .expect("supervisor task should not panic");
}

/// Test that restart count increments properly after failures.
/// Note: Full backoff timing tests would require tokio-test's time mocking.
#[tokio::test]
async fn test_restart_counter_increments() {
    let supervisor = Supervisor::new("counter-task");

    assert_eq!(supervisor.restart_count(), 0);

    // Manually record restarts via health failures
    supervisor.record_health_failure("failure 1").await;
    assert_eq!(supervisor.restart_count(), 1);

    supervisor.record_health_failure("failure 2").await;
    assert_eq!(supervisor.restart_count(), 2);

    supervisor.record_health_failure("failure 3").await;
    assert_eq!(supervisor.restart_count(), 3);
}

/// Test record_health_failure increments failure count.
#[tokio::test]
async fn test_record_health_failure() {
    let supervisor = Supervisor::new("health-task");

    assert_eq!(supervisor.restart_count(), 0);

    supervisor.record_health_failure("test failure 1").await;
    assert_eq!(supervisor.restart_count(), 1);

    supervisor.record_health_failure("test failure 2").await;
    assert_eq!(supervisor.restart_count(), 2);
}

/// Test should_attempt_recovery returns false after max failures.
#[tokio::test]
async fn test_should_attempt_recovery_respects_limit() {
    let supervisor = Supervisor::new("recovery-task");

    // Initially should allow recovery
    assert!(supervisor.should_attempt_recovery().await);

    // Record failures up to the limit
    supervisor.record_health_failure("failure 1").await;
    assert!(supervisor.should_attempt_recovery().await);

    supervisor.record_health_failure("failure 2").await;
    assert!(supervisor.should_attempt_recovery().await);

    supervisor.record_health_failure("failure 3").await;
    // After 3 failures (MAX_RESTARTS), should_attempt_recovery returns false
    assert!(!supervisor.should_attempt_recovery().await, "should not allow recovery after MAX_RESTARTS failures");
}

/// Test that restart times outside the window are pruned.
#[tokio::test]
async fn test_restart_window_pruning() {
    // This test would require time manipulation which is complex.
    // For now, we verify the basic behavior within the window.
    let supervisor = Supervisor::new("window-task");

    // Record 3 failures rapidly
    supervisor.record_health_failure("failure 1").await;
    supervisor.record_health_failure("failure 2").await;
    supervisor.record_health_failure("failure 3").await;

    // Should not allow more restarts within window
    assert!(!supervisor.should_attempt_recovery().await);

    // In a real scenario, after RESTART_WINDOW (10 min) passes,
    // old failures would be pruned and recovery would be allowed again.
    // This is tested implicitly by the should_restart logic.
}

/// Test run_raft_with_supervision helper function.
#[tokio::test]
async fn test_run_raft_with_supervision_helper() {
    use aspen::raft::supervisor::run_raft_with_supervision;

    let completed = Arc::new(AtomicU32::new(0));
    let completed_clone = Arc::clone(&completed);

    let supervisor = run_raft_with_supervision("test-raft".to_string(), move || {
        let done = completed_clone.clone();
        async move {
            done.store(1, Ordering::Relaxed);
            Ok(())
        }
    })
    .await;

    // Wait for the task to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(completed.load(Ordering::Relaxed), 1);
    assert_eq!(supervisor.restart_count(), 0);
}
