//! Durable timer implementation for workflows.
//!
//! This module provides crash-recoverable timers that are persisted to the
//! KV store. Timers survive node restarts and can be recovered by any node
//! in the cluster.
//!
//! ## Architecture
//!
//! Timers are stored in the KV store with keys:
//! - `__timers::{fire_at_ms:020}::{timer_id}` - Individual timer entries
//!
//! The zero-padded fire time ensures lexicographic ordering for efficient
//! time-ordered scans.
//!
//! ## Tiger Style
//!
//! - `MAX_TIMER_DURATION` = 1 year
//! - `MAX_PENDING_TIMERS` = 100,000
//! - `TIMER_SCAN_BATCH_SIZE` = 1,000

use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::ScanRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

use crate::error::JobError;
use crate::error::Result;
use crate::event_store::WorkflowExecutionId;

/// Key prefix for timer entries.
const TIMER_KEY_PREFIX: &str = "__timers::";

/// Maximum timer duration (1 year).
const MAX_TIMER_DURATION_MS: u64 = 365 * 24 * 60 * 60 * 1000;

/// Maximum number of pending timers per workflow.
const _MAX_TIMERS_PER_WORKFLOW: usize = 1_000;

/// Batch size for timer scans.
const TIMER_SCAN_BATCH_SIZE: u32 = 1_000;

/// How often to poll for ready timers.
const TIMER_POLL_INTERVAL_MS: u64 = 100;

/// Unique identifier for a timer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimerId(String);

impl TimerId {
    /// Create a new unique timer ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from an existing string.
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl Default for TimerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TimerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A durable timer entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurableTimer {
    /// Unique timer ID.
    pub timer_id: TimerId,
    /// Workflow this timer belongs to.
    pub workflow_id: WorkflowExecutionId,
    /// When the timer should fire (Unix timestamp milliseconds).
    pub fire_at_ms: u64,
    /// Duration from when the timer was scheduled.
    pub duration_ms: u64,
    /// When the timer was scheduled.
    pub scheduled_at: DateTime<Utc>,
    /// Optional callback data.
    pub callback_data: Option<serde_json::Value>,
    /// Whether the timer has fired.
    pub fired: bool,
}

impl DurableTimer {
    /// Generate the storage key for this timer.
    pub fn storage_key(&self) -> String {
        format!("{}{}::{}", TIMER_KEY_PREFIX, format_args!("{:020}", self.fire_at_ms), self.timer_id)
    }

    /// Parse a timer ID from a storage key.
    pub fn parse_key(key: &str) -> Option<(u64, TimerId)> {
        let key = key.strip_prefix(TIMER_KEY_PREFIX)?;
        let parts: Vec<&str> = key.split("::").collect();
        if parts.len() != 2 {
            return None;
        }
        let fire_at_ms = parts[0].parse().ok()?;
        let timer_id = TimerId::from_string(parts[1]);
        Some((fire_at_ms, timer_id))
    }
}

/// Manager for durable timers.
///
/// Handles timer scheduling, persistence, and firing.
pub struct DurableTimerManager<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + Send + Sync + 'static> DurableTimerManager<S> {
    /// Create a new timer manager.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    /// Schedule a new timer.
    pub async fn schedule_timer(
        &self,
        workflow_id: WorkflowExecutionId,
        duration: Duration,
        callback_data: Option<serde_json::Value>,
    ) -> Result<DurableTimer> {
        // Validate duration
        let duration_ms = duration.as_millis() as u64;
        if duration_ms > MAX_TIMER_DURATION_MS {
            return Err(JobError::InvalidJobSpec {
                reason: format!("Timer duration {} ms exceeds maximum {} ms", duration_ms, MAX_TIMER_DURATION_MS),
            });
        }

        // Calculate fire time
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis() as u64;
        let fire_at_ms = now + duration_ms;

        let timer = DurableTimer {
            timer_id: TimerId::new(),
            workflow_id,
            fire_at_ms,
            duration_ms,
            scheduled_at: Utc::now(),
            callback_data,
            fired: false,
        };

        // Persist the timer
        let key = timer.storage_key();
        let value = serde_json::to_string(&timer)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(JobError::from)?;

        debug!(
            timer_id = %timer.timer_id,
            workflow_id = %timer.workflow_id,
            fire_at_ms = timer.fire_at_ms,
            duration_ms = timer.duration_ms,
            "timer scheduled"
        );

        Ok(timer)
    }

    /// Schedule a timer to fire at a specific time.
    pub async fn schedule_timer_at(
        &self,
        workflow_id: WorkflowExecutionId,
        fire_at: DateTime<Utc>,
        callback_data: Option<serde_json::Value>,
    ) -> Result<DurableTimer> {
        let now = Utc::now();
        let duration = (fire_at - now).to_std().unwrap_or(Duration::ZERO);
        self.schedule_timer(workflow_id, duration, callback_data).await
    }

    /// Cancel a timer.
    pub async fn cancel_timer(&self, timer: &DurableTimer) -> Result<()> {
        let key = timer.storage_key();

        self.store
            .write(WriteRequest {
                command: WriteCommand::Delete { key },
            })
            .await
            .map_err(JobError::from)?;

        debug!(
            timer_id = %timer.timer_id,
            workflow_id = %timer.workflow_id,
            "timer cancelled"
        );

        Ok(())
    }

    /// Get a timer by ID.
    pub async fn get_timer(&self, fire_at_ms: u64, timer_id: &TimerId) -> Result<Option<DurableTimer>> {
        let key = format!("{}{}::{}", TIMER_KEY_PREFIX, format_args!("{:020}", fire_at_ms), timer_id);

        let response = match self.store.read(ReadRequest::new(key)).await {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(JobError::from(e)),
        };

        match response.kv {
            Some(kv) => {
                let timer: DurableTimer = serde_json::from_str(&kv.value)?;
                Ok(Some(timer))
            }
            None => Ok(None),
        }
    }

    /// Get all ready timers (fire_at <= now).
    pub async fn get_ready_timers(&self) -> Result<Vec<DurableTimer>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis() as u64;

        // Scan for all timers with fire_at <= now
        // Since keys are ordered by fire_at, we can scan the prefix
        let response = self
            .store
            .scan(ScanRequest {
                prefix: TIMER_KEY_PREFIX.to_string(),
                limit: Some(TIMER_SCAN_BATCH_SIZE),
                continuation_token: None,
            })
            .await
            .map_err(JobError::from)?;

        let mut ready = Vec::new();

        for kv in response.entries {
            if let Ok(timer) = serde_json::from_str::<DurableTimer>(&kv.value) {
                if timer.fire_at_ms <= now && !timer.fired {
                    ready.push(timer);
                } else if timer.fire_at_ms > now {
                    // Timers are ordered, so we can stop here
                    break;
                }
            }
        }

        Ok(ready)
    }

    /// Mark a timer as fired.
    pub async fn mark_fired(&self, timer: &DurableTimer) -> Result<()> {
        let mut updated = timer.clone();
        updated.fired = true;

        let key = timer.storage_key();
        let value = serde_json::to_string(&updated)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(JobError::from)?;

        debug!(
            timer_id = %timer.timer_id,
            workflow_id = %timer.workflow_id,
            "timer marked as fired"
        );

        Ok(())
    }

    /// Delete a fired timer (cleanup).
    pub async fn delete_fired_timer(&self, timer: &DurableTimer) -> Result<()> {
        if !timer.fired {
            return Err(JobError::InvalidJobState {
                state: "not_fired".to_string(),
                operation: "delete_fired_timer".to_string(),
            });
        }

        self.cancel_timer(timer).await
    }

    /// Get all timers for a workflow.
    pub async fn get_workflow_timers(&self, workflow_id: &WorkflowExecutionId) -> Result<Vec<DurableTimer>> {
        // We need to scan all timers and filter by workflow ID
        // This is not optimal but works for reasonable timer counts
        let response = self
            .store
            .scan(ScanRequest {
                prefix: TIMER_KEY_PREFIX.to_string(),
                limit: Some(TIMER_SCAN_BATCH_SIZE),
                continuation_token: None,
            })
            .await
            .map_err(JobError::from)?;

        let mut timers = Vec::new();

        for kv in response.entries {
            if let Ok(timer) = serde_json::from_str::<DurableTimer>(&kv.value) {
                if &timer.workflow_id == workflow_id {
                    timers.push(timer);
                }
            }
        }

        Ok(timers)
    }

    /// Cancel all timers for a workflow.
    pub async fn cancel_workflow_timers(&self, workflow_id: &WorkflowExecutionId) -> Result<usize> {
        let timers = self.get_workflow_timers(workflow_id).await?;
        let count = timers.len();

        for timer in timers {
            self.cancel_timer(&timer).await?;
        }

        info!(
            workflow_id = %workflow_id,
            cancelled_count = count,
            "cancelled workflow timers"
        );

        Ok(count)
    }
}

/// Event emitted when a timer fires.
#[derive(Debug, Clone)]
pub struct TimerFiredEvent {
    /// The timer that fired.
    pub timer: DurableTimer,
}

/// Background service that polls for and fires ready timers.
pub struct TimerService<S: KeyValueStore + ?Sized + Send + Sync + 'static> {
    manager: Arc<DurableTimerManager<S>>,
    fired_tx: mpsc::Sender<TimerFiredEvent>,
}

impl<S: KeyValueStore + ?Sized + Send + Sync + 'static> TimerService<S> {
    /// Create a new timer service.
    ///
    /// Returns the service and a receiver for timer fired events.
    pub fn new(store: Arc<S>) -> (Self, mpsc::Receiver<TimerFiredEvent>) {
        let (fired_tx, fired_rx) = mpsc::channel(1000);
        let manager = Arc::new(DurableTimerManager::new(store));

        (Self { manager, fired_tx }, fired_rx)
    }

    /// Get the timer manager.
    pub fn manager(&self) -> Arc<DurableTimerManager<S>> {
        Arc::clone(&self.manager)
    }

    /// Run the timer service.
    ///
    /// This polls for ready timers and emits events when they fire.
    /// Should be run in a background task.
    pub async fn run(self, mut shutdown: tokio::sync::broadcast::Receiver<()>) {
        let mut poll_interval = interval(Duration::from_millis(TIMER_POLL_INTERVAL_MS));

        info!("timer service started");

        loop {
            tokio::select! {
                _ = poll_interval.tick() => {
                    if let Err(e) = self.poll_timers().await {
                        error!(error = %e, "failed to poll timers");
                    }
                }
                _ = shutdown.recv() => {
                    info!("timer service shutting down");
                    break;
                }
            }
        }
    }

    async fn poll_timers(&self) -> Result<()> {
        let ready = self.manager.get_ready_timers().await?;

        for timer in ready {
            // Mark as fired first to prevent duplicate firing
            self.manager.mark_fired(&timer).await?;

            // Emit the event
            let event = TimerFiredEvent { timer: timer.clone() };

            if self.fired_tx.send(event).await.is_err() {
                warn!(
                    timer_id = %timer.timer_id,
                    "failed to send timer fired event - receiver dropped"
                );
            } else {
                debug!(
                    timer_id = %timer.timer_id,
                    workflow_id = %timer.workflow_id,
                    "timer fired"
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use aspen_core::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_timer_schedule_and_get() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let manager = DurableTimerManager::new(store);

        let workflow_id = WorkflowExecutionId::new();
        let timer = manager.schedule_timer(workflow_id.clone(), Duration::from_secs(60), None).await.unwrap();

        assert_eq!(timer.workflow_id, workflow_id);
        assert_eq!(timer.duration_ms, 60_000);
        assert!(!timer.fired);

        // Get the timer back
        let retrieved = manager.get_timer(timer.fire_at_ms, &timer.timer_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.timer_id, timer.timer_id);
    }

    #[tokio::test]
    async fn test_timer_cancel() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let manager = DurableTimerManager::new(store);

        let workflow_id = WorkflowExecutionId::new();
        let timer = manager.schedule_timer(workflow_id, Duration::from_secs(60), None).await.unwrap();

        // Cancel the timer
        manager.cancel_timer(&timer).await.unwrap();

        // Should no longer exist
        let retrieved = manager.get_timer(timer.fire_at_ms, &timer.timer_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_timer_mark_fired() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let manager = DurableTimerManager::new(store);

        let workflow_id = WorkflowExecutionId::new();
        let timer = manager.schedule_timer(workflow_id, Duration::from_secs(60), None).await.unwrap();

        // Mark as fired
        manager.mark_fired(&timer).await.unwrap();

        // Should be marked as fired
        let retrieved = manager.get_timer(timer.fire_at_ms, &timer.timer_id).await.unwrap().unwrap();
        assert!(retrieved.fired);
    }

    #[tokio::test]
    async fn test_get_ready_timers() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let manager = DurableTimerManager::new(store);

        let workflow_id = WorkflowExecutionId::new();

        // Schedule a timer in the past (should be ready)
        let past_timer = DurableTimer {
            timer_id: TimerId::new(),
            workflow_id: workflow_id.clone(),
            fire_at_ms: 0, // Unix epoch - definitely in the past
            duration_ms: 0,
            scheduled_at: Utc::now(),
            callback_data: None,
            fired: false,
        };

        // Persist directly
        let key = past_timer.storage_key();
        let value = serde_json::to_string(&past_timer).unwrap();
        manager
            .store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .unwrap();

        // Schedule a timer in the future (should not be ready)
        let _future_timer = manager.schedule_timer(workflow_id, Duration::from_secs(3600), None).await.unwrap();

        // Get ready timers
        let ready = manager.get_ready_timers().await.unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].timer_id, past_timer.timer_id);
    }

    #[tokio::test]
    async fn test_workflow_timers() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let manager = DurableTimerManager::new(store);

        let workflow_id1 = WorkflowExecutionId::new();
        let workflow_id2 = WorkflowExecutionId::new();

        // Schedule timers for different workflows
        let _ = manager.schedule_timer(workflow_id1.clone(), Duration::from_secs(60), None).await.unwrap();
        let _ = manager.schedule_timer(workflow_id1.clone(), Duration::from_secs(120), None).await.unwrap();
        let _ = manager.schedule_timer(workflow_id2.clone(), Duration::from_secs(60), None).await.unwrap();

        // Get timers for workflow 1
        let timers = manager.get_workflow_timers(&workflow_id1).await.unwrap();
        assert_eq!(timers.len(), 2);

        // Cancel all timers for workflow 1
        let cancelled = manager.cancel_workflow_timers(&workflow_id1).await.unwrap();
        assert_eq!(cancelled, 2);

        // Workflow 1 should have no timers
        let timers = manager.get_workflow_timers(&workflow_id1).await.unwrap();
        assert_eq!(timers.len(), 0);

        // Workflow 2 should still have 1 timer
        let timers = manager.get_workflow_timers(&workflow_id2).await.unwrap();
        assert_eq!(timers.len(), 1);
    }
}
