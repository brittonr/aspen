// Supervision system for critical VM tasks
//
// Implements supervision trees inspired by Erlang/OTP to ensure critical
// background tasks are automatically restarted on failure. This prevents
// silent failures where health checking or monitoring stops without notice.

use anyhow::{anyhow, Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep};

/// Supervision strategy for handling task failures
#[derive(Debug, Clone)]
pub enum SupervisionStrategy {
    /// Restart immediately on failure
    RestartAlways,
    /// Restart with exponential backoff
    RestartWithBackoff {
        initial_delay: Duration,
        max_delay: Duration,
        factor: f64,
    },
    /// Stop supervision after N failures
    RestartLimit {
        max_restarts: usize,
        within: Duration,
    },
}

/// Supervised task that can be restarted on failure
pub struct SupervisedTask {
    name: String,
    strategy: SupervisionStrategy,
    shutdown_rx: watch::Receiver<bool>,
    task_fn: Arc<dyn Fn() -> JoinHandle<()> + Send + Sync>,
    restart_count: usize,
    last_restart: Option<tokio::time::Instant>,
}

impl SupervisedTask {
    /// Create a new supervised task
    pub fn new<F>(
        name: String,
        strategy: SupervisionStrategy,
        shutdown_rx: watch::Receiver<bool>,
        task_fn: F,
    ) -> Self
    where
        F: Fn() -> JoinHandle<()> + Send + Sync + 'static,
    {
        Self {
            name,
            strategy,
            shutdown_rx,
            task_fn: Arc::new(task_fn),
            restart_count: 0,
            last_restart: None,
        }
    }

    /// Run the supervised task with automatic restarts
    pub async fn supervise(mut self) {
        let mut current_delay = Duration::from_secs(1);

        loop {
            // Check for shutdown signal
            if *self.shutdown_rx.borrow() {
                tracing::info!(task = %self.name, "Supervised task stopping due to shutdown signal");
                break;
            }

            // Check restart limits
            if !self.should_restart() {
                tracing::error!(
                    task = %self.name,
                    restarts = self.restart_count,
                    "Supervised task exceeded restart limit, stopping supervision"
                );
                break;
            }

            // Start the task
            let handle = (self.task_fn)();
            tracing::info!(
                task = %self.name,
                restart_count = self.restart_count,
                "Started supervised task"
            );

            // Wait for task completion or shutdown
            tokio::select! {
                result = handle => {
                    match result {
                        Ok(()) => {
                            tracing::warn!(
                                task = %self.name,
                                "Supervised task completed unexpectedly, will restart"
                            );
                        }
                        Err(e) => {
                            if e.is_panic() {
                                tracing::error!(
                                    task = %self.name,
                                    error = %e,
                                    "Supervised task panicked, will restart"
                                );
                            } else {
                                tracing::error!(
                                    task = %self.name,
                                    error = %e,
                                    "Supervised task failed, will restart"
                                );
                            }
                        }
                    }
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        tracing::info!(task = %self.name, "Supervised task received shutdown signal");
                        break;
                    }
                }
            }

            // Apply restart strategy
            self.restart_count += 1;
            self.last_restart = Some(tokio::time::Instant::now());

            let delay = self.calculate_restart_delay(&mut current_delay);
            if delay > Duration::ZERO {
                tracing::info!(
                    task = %self.name,
                    delay_secs = delay.as_secs(),
                    "Waiting before restarting supervised task"
                );
                sleep(delay).await;
            }
        }
    }

    /// Check if the task should be restarted based on strategy
    fn should_restart(&self) -> bool {
        match &self.strategy {
            SupervisionStrategy::RestartAlways => true,
            SupervisionStrategy::RestartWithBackoff { .. } => true,
            SupervisionStrategy::RestartLimit {
                max_restarts,
                within,
            } => {
                if self.restart_count >= *max_restarts {
                    if let Some(last) = self.last_restart {
                        // Reset counter if enough time has passed
                        if last.elapsed() > *within {
                            return true;
                        }
                    }
                    false
                } else {
                    true
                }
            }
        }
    }

    /// Calculate delay before next restart based on strategy
    fn calculate_restart_delay(&self, current_delay: &mut Duration) -> Duration {
        match &self.strategy {
            SupervisionStrategy::RestartAlways => Duration::from_secs(1),
            SupervisionStrategy::RestartWithBackoff {
                initial_delay,
                max_delay,
                factor,
            } => {
                let delay = if self.restart_count == 1 {
                    *initial_delay
                } else {
                    let new_delay = current_delay.mul_f64(*factor);
                    if new_delay > *max_delay {
                        *max_delay
                    } else {
                        new_delay
                    }
                };
                *current_delay = delay;
                delay
            }
            SupervisionStrategy::RestartLimit { .. } => Duration::from_secs(1),
        }
    }
}

/// Supervisor for managing multiple supervised tasks
pub struct TaskSupervisor {
    tasks: Vec<JoinHandle<()>>,
    shutdown_tx: watch::Sender<bool>,
}

impl TaskSupervisor {
    /// Create a new task supervisor
    pub fn new() -> Self {
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            tasks: Vec::new(),
            shutdown_tx,
        }
    }

    /// Add a task to be supervised
    pub fn supervise<F>(
        &mut self,
        name: String,
        strategy: SupervisionStrategy,
        task_fn: F,
    ) where
        F: Fn() -> JoinHandle<()> + Send + Sync + 'static,
    {
        let shutdown_rx = self.shutdown_tx.subscribe();
        let supervised = SupervisedTask::new(name.clone(), strategy, shutdown_rx, task_fn);

        let handle = tokio::spawn(async move {
            supervised.supervise().await;
        });

        self.tasks.push(handle);
        tracing::info!(task = %name, "Added task to supervisor");
    }

    /// Shutdown all supervised tasks
    pub async fn shutdown(&mut self) -> Result<()> {
        tracing::info!("Shutting down task supervisor");

        // Send shutdown signal to all tasks
        self.shutdown_tx
            .send(true)
            .context("Failed to send shutdown signal")?;

        // Wait for all tasks to complete
        for handle in self.tasks.drain(..) {
            // Give tasks time to shut down gracefully
            tokio::select! {
                _ = handle => {},
                _ = sleep(Duration::from_secs(10)) => {
                    tracing::warn!("Task did not shutdown gracefully, aborting");
                }
            }
        }

        tracing::info!("Task supervisor shutdown complete");
        Ok(())
    }
}

/// Helper function to create a supervised health checker task
pub fn supervised_health_checker(
    health_checker: Arc<super::health_checker::HealthChecker>,
) -> impl Fn() -> JoinHandle<()> + Send + Sync + 'static {
    move || {
        let health_checker = Arc::clone(&health_checker);
        tokio::spawn(async move {
            health_checker.health_check_loop().await;
        })
    }
}

/// Helper function to create a supervised resource monitor task
pub fn supervised_resource_monitor(
    monitor: Arc<super::resource_monitor::ResourceMonitor>,
) -> impl Fn() -> JoinHandle<()> + Send + Sync + 'static {
    move || {
        let monitor = Arc::clone(&monitor);
        tokio::spawn(async move {
            monitor.monitoring_loop().await;
        })
    }
}

/// Helper function to create a supervised consistency checker task
pub fn supervised_consistency_checker(
    registry: Arc<super::registry::DefaultVmRepository>,
    check_interval: Duration,
) -> impl Fn() -> JoinHandle<()> + Send + Sync + 'static {
    move || {
        let registry = Arc::clone(&registry);
        tokio::spawn(async move {
            let mut ticker = interval(check_interval);
            loop {
                ticker.tick().await;
                match registry.verify_consistency().await {
                    Ok(report) => {
                        if report.found > 0 {
                            tracing::warn!(
                                checked = report.checked,
                                found = report.found,
                                fixed = report.fixed,
                                "Consistency check found and fixed issues"
                            );
                        } else {
                            tracing::debug!(
                                checked = report.checked,
                                "Consistency check completed successfully"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Consistency check failed");
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_restart_always_strategy() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let mut supervisor = TaskSupervisor::new();

        supervisor.supervise(
            "test_task".to_string(),
            SupervisionStrategy::RestartAlways,
            move || {
                let counter = Arc::clone(&counter_clone);
                tokio::spawn(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    // Simulate quick failure
                    panic!("Task failed!");
                })
            },
        );

        // Let it run for a bit
        sleep(Duration::from_secs(3)).await;

        // Should have restarted multiple times
        let restarts = counter.load(Ordering::SeqCst);
        assert!(restarts >= 2, "Task should have restarted at least twice");

        supervisor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_restart_limit_strategy() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let mut supervisor = TaskSupervisor::new();

        supervisor.supervise(
            "limited_task".to_string(),
            SupervisionStrategy::RestartLimit {
                max_restarts: 2,
                within: Duration::from_secs(60),
            },
            move || {
                let counter = Arc::clone(&counter_clone);
                tokio::spawn(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    panic!("Task failed!");
                })
            },
        );

        // Let it run for a bit
        sleep(Duration::from_secs(5)).await;

        // Should have stopped after 2 restarts
        let restarts = counter.load(Ordering::SeqCst);
        assert_eq!(restarts, 3, "Task should run initially + 2 restarts = 3 times");

        supervisor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let mut supervisor = TaskSupervisor::new();

        supervisor.supervise(
            "long_running".to_string(),
            SupervisionStrategy::RestartAlways,
            || {
                tokio::spawn(async {
                    // Long running task
                    sleep(Duration::from_secs(100)).await;
                })
            },
        );

        // Start and then immediately shutdown
        sleep(Duration::from_millis(100)).await;
        supervisor.shutdown().await.unwrap();

        // Should complete without hanging
    }
}