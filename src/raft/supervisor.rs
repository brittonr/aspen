//! Simple supervision without actors.
//!
//! This module provides lightweight supervision for tokio tasks,
//! replacing the heavyweight actor-based supervision with simple
//! restart logic.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// Maximum restarts before giving up.
const MAX_RESTARTS: u32 = 3;

/// Time window for counting restarts (10 minutes).
const RESTART_WINDOW: Duration = Duration::from_secs(600);

/// Backoff durations for restarts.
const BACKOFF_DURATIONS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(5),
    Duration::from_secs(10),
];

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
        let mut times = self.restart_times.lock().await;
        let now = Instant::now();

        // Remove old restart times outside the window
        times.retain(|&t| now.duration_since(t) < RESTART_WINDOW);

        // Check if we've exceeded max restarts in the window
        times.len() < MAX_RESTARTS as usize
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
        let count = self.restart_count.load(Ordering::Acquire) as usize;
        let idx = count.min(BACKOFF_DURATIONS.len() - 1);
        BACKOFF_DURATIONS[idx]
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
