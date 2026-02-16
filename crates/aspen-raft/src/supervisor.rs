//! Simple supervision without actors.
//!
//! This module provides lightweight supervision for tokio tasks,
//! replacing the heavyweight actor-based supervision with simple
//! restart logic.
//!
//! # Test Coverage
//!
//! Unit tests in `#[cfg(test)]` module below cover:
//!   - Supervisor creation and initial state
//!   - Restart counting and backoff duration progression
//!   - should_restart() logic with window reset
//!   - Graceful shutdown via stop() and cancellation token
//!   - Health failure recording

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;
use tracing::warn;

/// Maximum restarts before giving up.
const MAX_RESTARTS: u32 = 3;

/// Time window for counting restarts (10 minutes).
const RESTART_WINDOW: Duration = Duration::from_secs(600);

/// Backoff durations for restarts.
const BACKOFF_DURATIONS: [Duration; 3] = [Duration::from_secs(1), Duration::from_secs(5), Duration::from_secs(10)];

/// Supervisor for async tasks.
///
/// Provides automatic restart with exponential backoff and circuit breaker.
pub struct Supervisor {
    /// Name of the supervised task (for logging).
    name: String,

    /// Number of restarts attempted.
    restart_count: AtomicU32,

    /// Timestamps of recent restarts (for rate limiting).
    restart_times: Mutex<Vec<Instant>>,

    /// Whether the supervisor should stop.
    stopping: AtomicBool,

    /// Cancellation token for graceful shutdown.
    cancel: CancellationToken,
}

impl Supervisor {
    /// Create a new supervisor.
    pub fn new(name: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            name: name.into(),
            restart_count: AtomicU32::new(0),
            restart_times: Mutex::new(Vec::new()),
            stopping: AtomicBool::new(false),
            cancel: CancellationToken::new(),
        })
    }

    /// Run a task with supervision.
    ///
    /// The task will be automatically restarted on failure with exponential backoff.
    /// After MAX_RESTARTS failures within RESTART_WINDOW, the supervisor gives up.
    pub async fn supervise<F, Fut>(self: Arc<Self>, mut task_factory: F)
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        while !self.stopping.load(Ordering::Acquire) {
            // Check if we should restart
            if !self.should_restart().await {
                error!(name = %self.name, "too many restarts, giving up");
                break;
            }

            info!(name = %self.name, "starting supervised task");

            // Run the task
            let result = tokio::select! {
                _ = self.cancel.cancelled() => {
                    info!(name = %self.name, "task cancelled");
                    break;
                }
                result = task_factory() => result,
            };

            // Handle result
            match result {
                Ok(()) => {
                    info!(name = %self.name, "task completed successfully");
                    break;
                }
                Err(err) => {
                    warn!(name = %self.name, error = %err, "task failed");

                    if self.stopping.load(Ordering::Acquire) {
                        break;
                    }

                    // Record restart
                    self.record_restart().await;

                    // Apply backoff
                    let backoff = self.get_backoff();
                    warn!(
                        name = %self.name,
                        backoff_secs = backoff.as_secs(),
                        "waiting before restart"
                    );
                    tokio::time::sleep(backoff).await;
                }
            }
        }

        info!(name = %self.name, "supervision ended");
    }

    /// Check if we should attempt a restart.
    async fn should_restart(&self) -> bool {
        use crate::verified::should_allow_restart;

        let mut times = self.restart_times.lock().await;
        let now = Instant::now();

        // Remove old restart times outside the window
        times.retain(|&t| now.duration_since(t) < RESTART_WINDOW);

        // Check if we've exceeded max restarts in the window (extracted pure logic)
        should_allow_restart(times.len().min(u32::MAX as usize) as u32, MAX_RESTARTS)
    }

    /// Record a restart attempt.
    async fn record_restart(&self) {
        let mut times = self.restart_times.lock().await;
        times.push(Instant::now());

        let count = self.restart_count.fetch_add(1, Ordering::AcqRel) + 1;
        warn!(
            name = %self.name,
            restart_count = count,
            "recording restart attempt"
        );
    }

    /// Get the backoff duration for the current restart count.
    fn get_backoff(&self) -> Duration {
        use crate::verified::calculate_backoff_duration;

        let count = self.restart_count.load(Ordering::Acquire) as usize;
        calculate_backoff_duration(count, &BACKOFF_DURATIONS)
    }

    /// Stop the supervisor.
    pub fn stop(&self) {
        self.stopping.store(true, Ordering::Release);
        self.cancel.cancel();
    }

    /// Get the number of restarts.
    pub fn restart_count(&self) -> u32 {
        self.restart_count.load(Ordering::Acquire)
    }

    /// Record an external health failure.
    ///
    /// This is called by health monitors to inform the supervisor of failures
    /// that might require intervention. Tracks failures for rate limiting.
    pub async fn record_health_failure(&self, reason: &str) {
        let mut times = self.restart_times.lock().await;
        times.push(Instant::now());

        let count = self.restart_count.fetch_add(1, Ordering::AcqRel) + 1;
        error!(
            name = %self.name,
            restart_count = count,
            reason = %reason,
            "health failure recorded"
        );
    }

    /// Check if we should take recovery action based on failure count.
    ///
    /// Returns true if we're within acceptable failure limits and should
    /// attempt recovery. Returns false if too many failures have occurred.
    pub async fn should_attempt_recovery(&self) -> bool {
        self.should_restart().await
    }

    /// Get cancellation token for coordinating shutdown.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Check if the supervisor is stopping.
    #[cfg(test)]
    pub fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::Acquire)
    }
}

/// Run a Raft node with supervision.
///
/// This replaces the actor-based supervision with a simpler approach.
pub async fn run_raft_with_supervision<F, Fut>(name: String, task_factory: F) -> Arc<Supervisor>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
{
    let supervisor = Supervisor::new(name);
    let supervisor_clone = supervisor.clone();

    tokio::spawn(async move {
        supervisor_clone.supervise(task_factory).await;
    });

    supervisor
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Supervisor Creation Tests
    // =========================================================================

    #[test]
    fn test_supervisor_new() {
        let supervisor = Supervisor::new("test-task");
        assert_eq!(supervisor.restart_count(), 0);
        assert!(!supervisor.is_stopping());
    }

    #[test]
    fn test_supervisor_name_into_string() {
        let supervisor = Supervisor::new(String::from("my-task"));
        assert_eq!(supervisor.restart_count(), 0);
    }

    #[test]
    fn test_supervisor_new_returns_arc() {
        let supervisor: Arc<Supervisor> = Supervisor::new("test");
        // Should be able to clone the Arc
        let _cloned = supervisor.clone();
        assert_eq!(supervisor.restart_count(), 0);
    }

    // =========================================================================
    // Backoff Duration Tests
    // =========================================================================

    #[test]
    fn test_backoff_durations_constant() {
        // Verify the backoff durations are as documented
        assert_eq!(BACKOFF_DURATIONS[0], Duration::from_secs(1));
        assert_eq!(BACKOFF_DURATIONS[1], Duration::from_secs(5));
        assert_eq!(BACKOFF_DURATIONS[2], Duration::from_secs(10));
    }

    #[test]
    fn test_get_backoff_initial() {
        let supervisor = Supervisor::new("test");
        // Initial backoff should be 1 second (index 0)
        assert_eq!(supervisor.get_backoff(), Duration::from_secs(1));
    }

    #[test]
    fn test_get_backoff_after_restarts() {
        let supervisor = Supervisor::new("test");

        // Initial: 1s
        assert_eq!(supervisor.get_backoff(), Duration::from_secs(1));

        // After 1 restart: 5s
        supervisor.restart_count.store(1, Ordering::Release);
        assert_eq!(supervisor.get_backoff(), Duration::from_secs(5));

        // After 2 restarts: 10s
        supervisor.restart_count.store(2, Ordering::Release);
        assert_eq!(supervisor.get_backoff(), Duration::from_secs(10));

        // After 3+ restarts: still 10s (capped at max)
        supervisor.restart_count.store(3, Ordering::Release);
        assert_eq!(supervisor.get_backoff(), Duration::from_secs(10));

        supervisor.restart_count.store(100, Ordering::Release);
        assert_eq!(supervisor.get_backoff(), Duration::from_secs(10));
    }

    // =========================================================================
    // Stop and Cancellation Tests
    // =========================================================================

    #[test]
    fn test_supervisor_stop() {
        let supervisor = Supervisor::new("test");
        assert!(!supervisor.is_stopping());

        supervisor.stop();

        assert!(supervisor.is_stopping());
        assert!(supervisor.cancel.is_cancelled());
    }

    #[test]
    fn test_cancellation_token() {
        let supervisor = Supervisor::new("test");
        let token = supervisor.cancellation_token();

        assert!(!token.is_cancelled());

        supervisor.stop();

        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_clones() {
        let supervisor = Supervisor::new("test");
        let token1 = supervisor.cancellation_token();
        let token2 = supervisor.cancellation_token();

        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());

        supervisor.stop();

        // All clones should be cancelled
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    // =========================================================================
    // Constants Tests
    // =========================================================================

    #[test]
    fn test_max_restarts_constant() {
        assert_eq!(MAX_RESTARTS, 3);
    }

    #[test]
    fn test_restart_window_constant() {
        assert_eq!(RESTART_WINDOW, Duration::from_secs(600));
    }

    // =========================================================================
    // Async Tests
    // =========================================================================

    #[tokio::test]
    async fn test_should_restart_initially_true() {
        let supervisor = Supervisor::new("test");
        // With no restarts, should_restart should return true
        assert!(supervisor.should_restart().await);
    }

    #[tokio::test]
    async fn test_record_restart_increments_count() {
        let supervisor = Supervisor::new("test");
        assert_eq!(supervisor.restart_count(), 0);

        supervisor.record_restart().await;
        assert_eq!(supervisor.restart_count(), 1);

        supervisor.record_restart().await;
        assert_eq!(supervisor.restart_count(), 2);

        supervisor.record_restart().await;
        assert_eq!(supervisor.restart_count(), 3);
    }

    #[tokio::test]
    async fn test_should_restart_false_after_max_restarts() {
        let supervisor = Supervisor::new("test");

        // Record MAX_RESTARTS restarts
        for _ in 0..MAX_RESTARTS {
            supervisor.record_restart().await;
        }

        // Now should_restart should return false
        assert!(!supervisor.should_restart().await);
    }

    #[tokio::test]
    async fn test_record_health_failure() {
        let supervisor = Supervisor::new("test");
        assert_eq!(supervisor.restart_count(), 0);

        supervisor.record_health_failure("test failure").await;
        assert_eq!(supervisor.restart_count(), 1);

        supervisor.record_health_failure("another failure").await;
        assert_eq!(supervisor.restart_count(), 2);
    }

    #[tokio::test]
    async fn test_should_attempt_recovery() {
        let supervisor = Supervisor::new("test");

        // Initially should allow recovery
        assert!(supervisor.should_attempt_recovery().await);

        // After MAX_RESTARTS, should not allow recovery
        for _ in 0..MAX_RESTARTS {
            supervisor.record_restart().await;
        }

        assert!(!supervisor.should_attempt_recovery().await);
    }

    #[tokio::test]
    async fn test_supervise_successful_task() {
        let supervisor = Supervisor::new("test");
        let supervisor_clone = supervisor.clone();

        // Create a task that succeeds immediately
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let handle = tokio::spawn(async move {
            supervisor_clone
                .supervise(move || {
                    let count = call_count_clone.clone();
                    async move {
                        count.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    }
                })
                .await;
        });

        // Wait for completion
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("should complete")
            .expect("should not panic");

        // Task should have been called once
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        // No restarts for successful task
        assert_eq!(supervisor.restart_count(), 0);
    }

    #[tokio::test]
    async fn test_supervise_cancelled_task() {
        let supervisor = Supervisor::new("test");
        let supervisor_clone = supervisor.clone();

        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        // Spawn supervision
        let handle = tokio::spawn(async move {
            supervisor_clone
                .supervise(move || {
                    let count = call_count_clone.clone();
                    async move {
                        count.fetch_add(1, Ordering::SeqCst);
                        // Hang forever
                        tokio::time::sleep(Duration::from_secs(3600)).await;
                        Ok(())
                    }
                })
                .await;
        });

        // Give it time to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel the supervisor
        supervisor.stop();

        // Should complete quickly
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("should complete after cancel")
            .expect("should not panic");

        // Task should have been started once
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_run_raft_with_supervision() {
        let supervisor = run_raft_with_supervision("test-raft".to_string(), || async {
            // Succeed immediately
            Ok(())
        })
        .await;

        // Give the spawned task time to complete
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Should have created a supervisor
        assert_eq!(supervisor.restart_count(), 0);
    }
}
