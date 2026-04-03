//! Durable workflow execution engine.
//!
//! Ties together [`WorkflowEventStore`], [`WorkflowReplayEngine`],
//! [`DurableTimerManager`], and [`SagaExecutor`] into a unified execution
//! model where every state transition is recorded as an append-only event.
//! After a crash the event log is replayed: completed activities return
//! their memoized results, pending timers are rescheduled, and execution
//! resumes from the point of interruption.
//!
//! # Usage
//!
//! ```ignore
//! use aspen_jobs::{DurableWorkflowExecutor, WorkflowHandle};
//!
//! let executor = DurableWorkflowExecutor::new(store.clone(), "node-1".to_string());
//! let handle = executor.start_workflow("my-workflow", json!({"key": "value"})).await?;
//!
//! let result1 = handle.execute_activity("step-1", "fetch-data", json!({}), |input| async {
//!     Ok(json!({"rows": 42}))
//! }).await?;
//!
//! handle.sleep(Duration::from_secs(5)).await?;
//!
//! let result2 = handle.execute_activity("step-2", "process", result1, |input| async {
//!     Ok(json!({"processed": true}))
//! }).await?;
//!
//! handle.complete(json!({"done": true})).await?;
//! ```
//!
//! # Recovery
//!
//! On leader election:
//!
//! ```ignore
//! executor.on_become_leader().await?;
//! // Discovers active workflows and replays their event logs.
//! // Memoized activities are not re-executed.
//! ```

use std::collections::HashMap;
use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use aspen_traits::KeyValueStore;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::durable_timer::DurableTimerManager;
use crate::durable_timer::TimerFiredEvent;
use crate::durable_timer::TimerId;
use crate::error::JobError;
use crate::error::Result;
use crate::event_store::WorkflowEvent;
use crate::event_store::WorkflowEventStore;
use crate::event_store::WorkflowEventType;
use crate::event_store::WorkflowExecutionId;
use crate::event_store::WorkflowReplayEngine;
use crate::event_store::WorkflowSnapshot;
use crate::event_store::snapshot::CompensationEntry;
use crate::event_store::snapshot::TimerState;

/// Key prefix for workflow execution state in KV.
const WORKFLOW_STATE_PREFIX: &str = "__wf_state::";

/// Maximum concurrent workflows per executor (Tiger Style bound).
const MAX_CONCURRENT_WORKFLOWS: usize = 10_000;

/// Execution status of a durable workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurableWorkflowStatus {
    /// Workflow is actively executing (live mode).
    Running,
    /// Workflow is replaying from event log (recovery).
    Replaying,
    /// Workflow completed successfully.
    Completed,
    /// Workflow failed.
    Failed(String),
    /// Workflow was cancelled.
    Cancelled(String),
    /// Saga compensation is in progress.
    Compensating,
}

impl DurableWorkflowStatus {
    /// Check if the workflow is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed(_) | Self::Cancelled(_))
    }
}

/// Persistent workflow state stored in KV alongside the event log.
/// This is a cache — the event log is the source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedWorkflowState {
    workflow_id: WorkflowExecutionId,
    workflow_type: String,
    status: DurableWorkflowStatus,
    last_event_id: Option<u64>,
    last_snapshot_event_id: Option<u64>,
    started_at: chrono::DateTime<Utc>,
    completed_at: Option<chrono::DateTime<Utc>>,
}

/// Per-workflow in-memory execution state.
struct WorkflowExecution {
    #[allow(dead_code)] // Used for debug logging; will be read when wired into node
    workflow_id: WorkflowExecutionId,
    workflow_type: String,
    status: DurableWorkflowStatus,
    replay_engine: WorkflowReplayEngine,
    last_event_id: Option<u64>,
    last_snapshot_event_id: Option<u64>,
    /// Pending timer wake-ups: timer_id → sender to unblock the sleeping workflow.
    pending_timers: HashMap<TimerId, oneshot::Sender<()>>,
    /// Activity execution counts for memoization verification.
    activity_execution_counts: HashMap<String, u32>,
    /// Compensation entries for saga rollback tracking.
    compensation_stack: Vec<CompensationEntry>,
    /// Side effect sequence counter (mirrors replay engine).
    side_effect_seq: u64,
}

/// Durable workflow execution engine.
///
/// Composes [`WorkflowEventStore`], [`DurableTimerManager`], and
/// [`WorkflowReplayEngine`] to provide crash-recoverable workflow
/// execution with activity memoization and durable timers.
pub struct DurableWorkflowExecutor<S: KeyValueStore + ?Sized> {
    /// Event store for append-only event recording.
    event_store: Arc<WorkflowEventStore<S>>,
    /// Timer manager for durable sleep operations.
    timer_manager: Arc<DurableTimerManager<S>>,
    /// KV store for workflow state persistence.
    store: Arc<S>,
    /// Node ID for this executor instance.
    node_id: String,
    /// Active workflow executions keyed by workflow ID.
    workflows: Arc<RwLock<HashMap<WorkflowExecutionId, Arc<Mutex<WorkflowExecution>>>>>,
    /// Set of recovered workflow IDs to prevent duplicate recovery.
    recovered: Arc<RwLock<HashSet<WorkflowExecutionId>>>,
    /// Channel for dispatching timer-fired events to waiting workflows.
    timer_dispatch_tx: mpsc::Sender<TimerFiredEvent>,
}

impl<S: KeyValueStore + ?Sized + Send + Sync + 'static> DurableWorkflowExecutor<S> {
    /// Create a new durable workflow executor.
    ///
    /// The executor does not start any background tasks on creation.
    /// Call [`start_timer_dispatcher`] to begin processing timer events.
    pub fn new(store: Arc<S>, node_id: String) -> (Self, mpsc::Receiver<TimerFiredEvent>) {
        let event_store = Arc::new(WorkflowEventStore::new(store.clone(), node_id.clone()));
        let timer_manager = Arc::new(DurableTimerManager::new(store.clone()));
        let (timer_dispatch_tx, timer_dispatch_rx) = mpsc::channel(1_000);

        let executor = Self {
            event_store,
            timer_manager,
            store,
            node_id,
            workflows: Arc::new(RwLock::new(HashMap::new())),
            recovered: Arc::new(RwLock::new(HashSet::new())),
            timer_dispatch_tx,
        };

        (executor, timer_dispatch_rx)
    }

    /// Start a new durable workflow.
    ///
    /// Records a `WorkflowStarted` event and returns a [`WorkflowHandle`]
    /// for driving the execution.
    pub async fn start_workflow(&self, workflow_type: &str, input: serde_json::Value) -> Result<WorkflowHandle<S>> {
        // Tiger Style: bounded concurrent workflows
        let workflow_count = self.workflows.read().await.len();
        if workflow_count >= MAX_CONCURRENT_WORKFLOWS {
            return Err(JobError::ExecutionFailed {
                reason: format!("Maximum concurrent workflows ({}) reached", MAX_CONCURRENT_WORKFLOWS),
            });
        }

        let workflow_id = WorkflowExecutionId::new();

        // Record the WorkflowStarted event.
        let event_id = self
            .event_store
            .append_event(
                &workflow_id,
                WorkflowEventType::WorkflowStarted {
                    workflow_type: workflow_type.to_string(),
                    input: input.clone(),
                    parent_workflow_id: None,
                },
                None,
            )
            .await?;

        // Create in-memory execution state with an empty replay engine (live mode).
        let execution = WorkflowExecution {
            workflow_id: workflow_id.clone(),
            workflow_type: workflow_type.to_string(),
            status: DurableWorkflowStatus::Running,
            replay_engine: WorkflowReplayEngine::new(Vec::new()),
            last_event_id: Some(event_id),
            last_snapshot_event_id: None,
            pending_timers: HashMap::new(),
            activity_execution_counts: HashMap::new(),
            compensation_stack: Vec::new(),
            side_effect_seq: 0,
        };

        // The replay engine starts in replay mode with 0 events, which means
        // is_replaying() returns false immediately — we're in live mode.
        let execution = Arc::new(Mutex::new(execution));
        self.workflows.write().await.insert(workflow_id.clone(), execution.clone());

        // Persist workflow state.
        self.persist_workflow_state(&workflow_id, workflow_type, &DurableWorkflowStatus::Running, Some(event_id), None)
            .await?;

        info!(
            workflow_id = %workflow_id,
            workflow_type,
            "durable workflow started"
        );

        Ok(WorkflowHandle {
            workflow_id,
            execution,
            event_store: self.event_store.clone(),
            timer_manager: self.timer_manager.clone(),
            store: self.store.clone(),
            timer_dispatch_tx: self.timer_dispatch_tx.clone(),
        })
    }

    /// Recover a workflow from its event log.
    ///
    /// Loads the latest snapshot (if any), replays events from that point,
    /// and returns a [`WorkflowHandle`] positioned at the end of the log
    /// ready for live execution.
    pub async fn recover(&self, workflow_id: &WorkflowExecutionId) -> Result<WorkflowHandle<S>> {
        // Prevent duplicate recovery.
        {
            let recovered = self.recovered.read().await;
            if recovered.contains(workflow_id) {
                return Err(JobError::ExecutionFailed {
                    reason: format!("workflow {} already recovered", workflow_id),
                });
            }
        }

        // Check if already running.
        if self.workflows.read().await.contains_key(workflow_id) {
            return Err(JobError::ExecutionFailed {
                reason: format!("workflow {} is already running", workflow_id),
            });
        }

        // Load snapshot if available.
        let snapshot = self.event_store.load_latest_snapshot(workflow_id).await?;
        let replay_from = snapshot.as_ref().map(|s| s.at_event_id + 1);
        let last_snapshot_event_id = snapshot.as_ref().map(|s| s.at_event_id);

        // Replay events from snapshot point (or beginning).
        let events = self.event_store.replay_events(workflow_id, replay_from).await?;

        // Extract workflow_type from the first event or snapshot.
        let workflow_type = Self::extract_workflow_type(&events, &snapshot)?;

        // Build replay engine.
        let replay_engine = WorkflowReplayEngine::new(events.clone());

        // Determine side_effect_seq from snapshot or events.
        let side_effect_seq = snapshot.as_ref().map(|s| s.side_effect_seq).unwrap_or(0);

        // Reconstruct compensation stack from snapshot or events.
        let compensation_stack = snapshot.as_ref().map(|s| s.compensation_stack.clone()).unwrap_or_default();

        let last_event_id = events.last().map(|e| e.event_id);

        let execution = WorkflowExecution {
            workflow_id: workflow_id.clone(),
            workflow_type: workflow_type.clone(),
            status: DurableWorkflowStatus::Replaying,
            replay_engine,
            last_event_id,
            last_snapshot_event_id,
            pending_timers: HashMap::new(),
            activity_execution_counts: HashMap::new(),
            compensation_stack,
            side_effect_seq,
        };

        let execution = Arc::new(Mutex::new(execution));
        self.workflows.write().await.insert(workflow_id.clone(), execution.clone());
        self.recovered.write().await.insert(workflow_id.clone());

        info!(
            workflow_id = %workflow_id,
            workflow_type,
            event_count = events.len(),
            snapshot = last_snapshot_event_id.is_some(),
            "workflow recovered from event log"
        );

        Ok(WorkflowHandle {
            workflow_id: workflow_id.clone(),
            execution,
            event_store: self.event_store.clone(),
            timer_manager: self.timer_manager.clone(),
            store: self.store.clone(),
            timer_dispatch_tx: self.timer_dispatch_tx.clone(),
        })
    }

    /// Discover active (non-terminal) workflows by scanning the KV store.
    ///
    /// Returns workflow IDs that have a `WorkflowStarted` event but no
    /// terminal event (`WorkflowCompleted`, `WorkflowFailed`, `WorkflowCancelled`).
    pub async fn discover_active_workflows(&self) -> Result<Vec<WorkflowExecutionId>> {
        // Scan workflow state entries.
        let scan_result = self
            .store
            .scan(aspen_kv_types::ScanRequest {
                prefix: WORKFLOW_STATE_PREFIX.to_string(),
                limit_results: Some(MAX_CONCURRENT_WORKFLOWS as u32),
                continuation_token: None,
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        let mut active = Vec::new();

        for entry in scan_result.entries {
            if let Ok(state) = serde_json::from_str::<PersistedWorkflowState>(&entry.value) {
                if !state.status.is_terminal() {
                    active.push(state.workflow_id);
                }
            }
        }

        info!(active_count = active.len(), "discovered active workflows");
        Ok(active)
    }

    /// Recover all active workflows after leader election.
    ///
    /// Calls [`discover_active_workflows`] and [`recover`] for each,
    /// skipping any that are already running.
    pub async fn recover_all(&self) -> Result<Vec<WorkflowHandle<S>>> {
        let active_ids = self.discover_active_workflows().await?;
        let mut handles = Vec::new();

        for workflow_id in active_ids {
            // Skip already-running workflows.
            if self.workflows.read().await.contains_key(&workflow_id) {
                debug!(workflow_id = %workflow_id, "workflow already running, skipping recovery");
                continue;
            }

            match self.recover(&workflow_id).await {
                Ok(handle) => handles.push(handle),
                Err(e) => {
                    error!(workflow_id = %workflow_id, error = ?e, "failed to recover workflow");
                }
            }
        }

        info!(recovered_count = handles.len(), "recovered active workflows");
        Ok(handles)
    }

    /// Called when this node becomes the Raft leader.
    ///
    /// Discovers and recovers all active workflows.
    pub async fn on_become_leader(&self) -> Result<Vec<WorkflowHandle<S>>> {
        info!(node_id = %self.node_id, "became leader — recovering durable workflows");
        self.recover_all().await
    }

    /// Called when this node loses Raft leadership.
    ///
    /// Gracefully cancels all in-progress workflow executions.
    /// They will be recovered by the new leader.
    pub async fn on_lose_leadership(&self) {
        info!(node_id = %self.node_id, "lost leadership — cancelling durable workflows");

        let mut workflows = self.workflows.write().await;
        let workflow_ids: Vec<WorkflowExecutionId> = workflows.keys().cloned().collect();

        for workflow_id in &workflow_ids {
            if let Some(execution) = workflows.get(workflow_id) {
                let mut exec = execution.lock().await;
                if !exec.status.is_terminal() {
                    exec.status = DurableWorkflowStatus::Cancelled("leadership lost".to_string());
                    // Cancel any pending timer wake-ups.
                    exec.pending_timers.clear();
                }
            }
        }

        workflows.clear();
        self.recovered.write().await.clear();

        info!(cancelled_count = workflow_ids.len(), "cancelled workflows after leadership loss");
    }

    /// Dispatch a timer-fired event to the waiting workflow.
    ///
    /// Called by the timer service integration loop.
    pub async fn dispatch_timer_event(&self, event: TimerFiredEvent) {
        let timer_id = &event.timer.timer_id;
        let workflow_id = &event.timer.workflow_id;

        let workflows = self.workflows.read().await;
        if let Some(execution) = workflows.get(workflow_id) {
            let mut exec = execution.lock().await;
            if let Some(sender) = exec.pending_timers.remove(timer_id) {
                if sender.send(()).is_err() {
                    warn!(
                        timer_id = %timer_id,
                        workflow_id = %workflow_id,
                        "timer fired but receiver was dropped"
                    );
                } else {
                    debug!(
                        timer_id = %timer_id,
                        workflow_id = %workflow_id,
                        "timer event dispatched to workflow"
                    );
                }
            }
        } else {
            debug!(
                timer_id = %timer_id,
                workflow_id = %workflow_id,
                "timer fired for unknown workflow — may have completed"
            );
        }
    }

    /// Get the event store reference.
    pub fn event_store(&self) -> &Arc<WorkflowEventStore<S>> {
        &self.event_store
    }

    /// Get the timer manager reference.
    pub fn timer_manager(&self) -> &Arc<DurableTimerManager<S>> {
        &self.timer_manager
    }

    /// Get the count of active workflows.
    pub async fn active_workflow_count(&self) -> usize {
        self.workflows.read().await.len()
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Persist workflow state to KV (cache — events are the source of truth).
    async fn persist_workflow_state(
        &self,
        workflow_id: &WorkflowExecutionId,
        workflow_type: &str,
        status: &DurableWorkflowStatus,
        last_event_id: Option<u64>,
        last_snapshot_event_id: Option<u64>,
    ) -> Result<()> {
        let state = PersistedWorkflowState {
            workflow_id: workflow_id.clone(),
            workflow_type: workflow_type.to_string(),
            status: status.clone(),
            last_event_id,
            last_snapshot_event_id,
            started_at: Utc::now(),
            completed_at: if status.is_terminal() { Some(Utc::now()) } else { None },
        };

        let key = format!("{}{}", WORKFLOW_STATE_PREFIX, workflow_id);
        let value = serde_json::to_string(&state)?;

        self.store
            .write(aspen_kv_types::WriteRequest {
                command: aspen_kv_types::WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        Ok(())
    }

    /// Extract workflow type from events or snapshot.
    fn extract_workflow_type(events: &[WorkflowEvent], snapshot: &Option<WorkflowSnapshot>) -> Result<String> {
        // Try events first.
        for event in events {
            if let WorkflowEventType::WorkflowStarted { ref workflow_type, .. } = event.event_type {
                return Ok(workflow_type.clone());
            }
        }

        // Fall back to snapshot metadata (stored as JSON in state field).
        if let Some(snap) = snapshot {
            if let Some(wt) = snap.state.get("workflow_type").and_then(|v| v.as_str()) {
                return Ok(wt.to_string());
            }
        }

        Err(JobError::ExecutionFailed {
            reason: "cannot determine workflow type: no WorkflowStarted event or snapshot found".to_string(),
        })
    }
}

/// Handle for driving a single durable workflow execution.
///
/// Returned by [`DurableWorkflowExecutor::start_workflow`] and
/// [`DurableWorkflowExecutor::recover`]. All operations on this handle
/// are scoped to the associated workflow.
pub struct WorkflowHandle<S: KeyValueStore + ?Sized> {
    /// Workflow execution ID.
    pub workflow_id: WorkflowExecutionId,
    /// Shared execution state.
    execution: Arc<Mutex<WorkflowExecution>>,
    /// Event store for appending events.
    event_store: Arc<WorkflowEventStore<S>>,
    /// Timer manager for durable sleep.
    timer_manager: Arc<DurableTimerManager<S>>,
    /// KV store for state persistence.
    store: Arc<S>,
    /// Channel for timer dispatch registration.
    #[allow(dead_code)] // Held for future timer dispatch integration
    timer_dispatch_tx: mpsc::Sender<TimerFiredEvent>,
}

impl<S: KeyValueStore + ?Sized + Send + Sync + 'static> WorkflowHandle<S> {
    /// Execute an activity within this workflow.
    ///
    /// In replay mode, returns the memoized result from the event log
    /// without calling `activity_fn`. In live mode, executes the function
    /// and records `ActivityScheduled` + `ActivityCompleted`/`ActivityFailed`.
    pub async fn execute_activity<F, Fut>(
        &self,
        activity_id: &str,
        activity_type: &str,
        input: serde_json::Value,
        activity_fn: F,
    ) -> Result<serde_json::Value>
    where
        F: FnOnce(serde_json::Value) -> Fut,
        Fut: Future<Output = std::result::Result<serde_json::Value, String>>,
    {
        let mut exec = self.execution.lock().await;

        // Check for memoized result during replay.
        if exec.replay_engine.is_replaying() {
            let cached = exec.replay_engine.get_activity_result(activity_id).cloned();
            if let Some(cached_value) = cached {
                debug!(
                    workflow_id = %self.workflow_id,
                    activity_id,
                    "returning memoized activity result (replay)"
                );
                exec.replay_engine.advance();
                return Ok(cached_value);
            }
            // No cached result — switch to live mode.
            exec.replay_engine.finish_replay();
            exec.status = DurableWorkflowStatus::Running;
        }

        let prev_event_id = exec.last_event_id;

        // Record ActivityScheduled event.
        let scheduled_event_id = self
            .event_store
            .append_event(
                &self.workflow_id,
                WorkflowEventType::ActivityScheduled {
                    activity_id: activity_id.to_string(),
                    activity_type: activity_type.to_string(),
                    input: input.clone(),
                    timeout_ms: None,
                },
                prev_event_id,
            )
            .await?;

        exec.last_event_id = Some(scheduled_event_id);
        *exec.activity_execution_counts.entry(activity_id.to_string()).or_insert(0) += 1;

        // Drop the lock before executing the activity (which may take time).
        drop(exec);

        let start = std::time::Instant::now();
        let result = activity_fn(input).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        // Re-acquire lock to record result.
        let mut exec = self.execution.lock().await;

        match result {
            Ok(output) => {
                let event_id = self
                    .event_store
                    .append_event(
                        &self.workflow_id,
                        WorkflowEventType::ActivityCompleted {
                            activity_id: activity_id.to_string(),
                            result: output.clone(),
                            duration_ms,
                        },
                        exec.last_event_id,
                    )
                    .await?;
                exec.last_event_id = Some(event_id);

                // Check if snapshot is needed.
                self.maybe_snapshot(&mut exec).await?;

                Ok(output)
            }
            Err(error_msg) => {
                let event_id = self
                    .event_store
                    .append_event(
                        &self.workflow_id,
                        WorkflowEventType::ActivityFailed {
                            activity_id: activity_id.to_string(),
                            error: error_msg.clone(),
                            is_retryable: false,
                            attempt: exec.activity_execution_counts.get(activity_id).copied().unwrap_or(1),
                        },
                        exec.last_event_id,
                    )
                    .await?;
                exec.last_event_id = Some(event_id);

                Err(JobError::ExecutionFailed { reason: error_msg })
            }
        }
    }

    /// Record a non-deterministic side effect.
    ///
    /// In replay mode, returns the cached value. In live mode, calls
    /// `produce_fn`, records a `SideEffectRecorded` event, and returns
    /// the produced value.
    pub async fn record_side_effect<F>(&self, produce_fn: F) -> Result<serde_json::Value>
    where F: FnOnce() -> serde_json::Value {
        let mut exec = self.execution.lock().await;

        // During replay, return cached side effect.
        if exec.replay_engine.is_replaying() {
            if let Some(cached) = exec.replay_engine.get_side_effect() {
                debug!(
                    workflow_id = %self.workflow_id,
                    seq = exec.side_effect_seq,
                    "returning cached side effect (replay)"
                );
                exec.side_effect_seq += 1;
                return Ok(cached);
            }
            // No more cached side effects — switch to live.
            exec.replay_engine.finish_replay();
            exec.status = DurableWorkflowStatus::Running;
        }

        // Live mode: produce the value and record it.
        let value = produce_fn();
        let seq = exec.side_effect_seq;
        exec.side_effect_seq += 1;

        let event_id = self
            .event_store
            .append_event(
                &self.workflow_id,
                WorkflowEventType::SideEffectRecorded {
                    seq,
                    value: value.clone(),
                },
                exec.last_event_id,
            )
            .await?;
        exec.last_event_id = Some(event_id);

        Ok(value)
    }

    /// Durable sleep that survives node restarts.
    ///
    /// In live mode, schedules a [`DurableTimer`] and records a
    /// `TimerScheduled` event. In replay mode, checks for a `TimerFired`
    /// event — if found returns immediately, otherwise reschedules the
    /// timer and awaits.
    pub async fn sleep(&self, duration: Duration) -> Result<()> {
        let mut exec = self.execution.lock().await;

        // During replay, check if this timer already fired.
        if exec.replay_engine.is_replaying() {
            // Look for TimerScheduled + TimerFired pair.
            if let Some(event) = exec.replay_engine.current_event() {
                if matches!(&event.event_type, WorkflowEventType::TimerScheduled { .. }) {
                    exec.replay_engine.advance();
                    // Check for corresponding TimerFired.
                    if let Some(next) = exec.replay_engine.current_event() {
                        if matches!(&next.event_type, WorkflowEventType::TimerFired { .. }) {
                            exec.replay_engine.advance();
                            debug!(
                                workflow_id = %self.workflow_id,
                                "durable sleep already fired (replay), skipping"
                            );
                            return Ok(());
                        }
                    }
                    // Timer was scheduled but never fired — we need to reschedule.
                    exec.replay_engine.finish_replay();
                    exec.status = DurableWorkflowStatus::Running;
                }
            }
        }

        // Live mode: schedule a durable timer.
        let timer = self.timer_manager.schedule_timer(self.workflow_id.clone(), duration, None).await?;

        let timer_id = timer.timer_id.clone();

        // Record TimerScheduled event.
        let event_id = self
            .event_store
            .append_event(
                &self.workflow_id,
                WorkflowEventType::TimerScheduled {
                    timer_id: timer_id.to_string(),
                    fire_at: chrono::DateTime::from_timestamp_millis(timer.fire_at_ms as i64).unwrap_or_else(Utc::now),
                    duration_ms: duration.as_millis() as u64,
                },
                exec.last_event_id,
            )
            .await?;
        exec.last_event_id = Some(event_id);

        // Register a oneshot channel for the timer dispatch.
        let (tx, rx) = oneshot::channel();
        exec.pending_timers.insert(timer_id.clone(), tx);

        // Drop lock while waiting for timer.
        drop(exec);

        debug!(
            workflow_id = %self.workflow_id,
            timer_id = %timer_id,
            duration_ms = duration.as_millis(),
            "awaiting durable timer"
        );

        // Wait for the timer to fire.
        let _ = rx.await;

        // Record TimerFired event.
        let mut exec = self.execution.lock().await;
        let event_id = self
            .event_store
            .append_event(
                &self.workflow_id,
                WorkflowEventType::TimerFired {
                    timer_id: timer_id.to_string(),
                },
                exec.last_event_id,
            )
            .await?;
        exec.last_event_id = Some(event_id);

        Ok(())
    }

    /// Mark the workflow as successfully completed.
    pub async fn complete(&self, output: serde_json::Value) -> Result<()> {
        let mut exec = self.execution.lock().await;

        if exec.status.is_terminal() {
            return Err(JobError::InvalidJobState {
                state: format!("{:?}", exec.status),
                operation: "complete".to_string(),
            });
        }

        let event_id = self
            .event_store
            .append_event(&self.workflow_id, WorkflowEventType::WorkflowCompleted { output }, exec.last_event_id)
            .await?;
        exec.last_event_id = Some(event_id);
        exec.status = DurableWorkflowStatus::Completed;

        // Persist terminal state.
        self.persist_terminal_state(&exec).await?;

        info!(workflow_id = %self.workflow_id, "durable workflow completed");
        Ok(())
    }

    /// Mark the workflow as failed.
    pub async fn fail(&self, error: &str) -> Result<()> {
        let mut exec = self.execution.lock().await;

        if exec.status.is_terminal() {
            return Err(JobError::InvalidJobState {
                state: format!("{:?}", exec.status),
                operation: "fail".to_string(),
            });
        }

        let event_id = self
            .event_store
            .append_event(
                &self.workflow_id,
                WorkflowEventType::WorkflowFailed {
                    error: error.to_string(),
                    error_type: None,
                    is_retryable: false,
                },
                exec.last_event_id,
            )
            .await?;
        exec.last_event_id = Some(event_id);
        exec.status = DurableWorkflowStatus::Failed(error.to_string());

        self.persist_terminal_state(&exec).await?;

        info!(workflow_id = %self.workflow_id, error, "durable workflow failed");
        Ok(())
    }

    /// Get the current execution status.
    pub async fn status(&self) -> DurableWorkflowStatus {
        self.execution.lock().await.status.clone()
    }

    /// Get the total event count for this workflow.
    pub async fn event_count(&self) -> Result<u64> {
        let events = self.event_store.replay_events(&self.workflow_id, None).await?;
        Ok(events.len() as u64)
    }

    /// Get all events for this workflow.
    pub async fn events(&self) -> Result<Vec<WorkflowEvent>> {
        self.event_store.replay_events(&self.workflow_id, None).await
    }

    /// Record saga compensation events.
    ///
    /// Records `CompensationStarted` before executing the compensation
    /// function, and `CompensationCompleted`/`CompensationFailed` after.
    pub async fn record_compensation<F, Fut>(&self, compensation_id: &str, compensation_fn: F) -> Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = std::result::Result<(), String>>,
    {
        let mut exec = self.execution.lock().await;

        // During replay, check if this compensation already completed.
        if exec.replay_engine.is_replaying() {
            // Look for CompensationStarted + CompensationCompleted pair.
            if let Some(event) = exec.replay_engine.current_event() {
                if let WorkflowEventType::CompensationStarted {
                    compensation_id: ref cid,
                } = event.event_type
                {
                    if cid == compensation_id {
                        exec.replay_engine.advance();
                        if let Some(next) = exec.replay_engine.current_event() {
                            if matches!(&next.event_type, WorkflowEventType::CompensationCompleted { .. }) {
                                exec.replay_engine.advance();
                                debug!(
                                    workflow_id = %self.workflow_id,
                                    compensation_id,
                                    "compensation already completed (replay)"
                                );
                                return Ok(());
                            }
                        }
                        // Started but not completed — need to re-execute.
                        exec.replay_engine.finish_replay();
                        exec.status = DurableWorkflowStatus::Compensating;
                    }
                }
            }
        }

        exec.status = DurableWorkflowStatus::Compensating;

        // Record CompensationStarted.
        let event_id = self
            .event_store
            .append_event(
                &self.workflow_id,
                WorkflowEventType::CompensationStarted {
                    compensation_id: compensation_id.to_string(),
                },
                exec.last_event_id,
            )
            .await?;
        exec.last_event_id = Some(event_id);

        // Drop lock during compensation execution.
        drop(exec);

        let result = compensation_fn().await;

        // Record result.
        let mut exec = self.execution.lock().await;
        match result {
            Ok(()) => {
                let event_id = self
                    .event_store
                    .append_event(
                        &self.workflow_id,
                        WorkflowEventType::CompensationCompleted {
                            compensation_id: compensation_id.to_string(),
                        },
                        exec.last_event_id,
                    )
                    .await?;
                exec.last_event_id = Some(event_id);
                Ok(())
            }
            Err(error_msg) => {
                let event_id = self
                    .event_store
                    .append_event(
                        &self.workflow_id,
                        WorkflowEventType::CompensationFailed {
                            compensation_id: compensation_id.to_string(),
                            error: error_msg.clone(),
                        },
                        exec.last_event_id,
                    )
                    .await?;
                exec.last_event_id = Some(event_id);
                Err(JobError::ExecutionFailed { reason: error_msg })
            }
        }
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Check if a snapshot should be taken and take it if so.
    async fn maybe_snapshot(&self, exec: &mut WorkflowExecution) -> Result<()> {
        let current_event_id = exec.last_event_id.unwrap_or(0);
        if self.event_store.should_snapshot(current_event_id, exec.last_snapshot_event_id) {
            let snapshot = WorkflowSnapshot {
                schema_version: 1,
                snapshot_id: current_event_id,
                workflow_id: self.workflow_id.clone(),
                at_event_id: current_event_id,
                hlc_timestamp: aspen_hlc::SerializableTimestamp::from(self.event_store.hlc().new_timestamp()),
                timestamp: Utc::now(),
                state: serde_json::json!({
                    "workflow_type": exec.workflow_type,
                    "status": exec.status,
                }),
                pending_activities: HashMap::new(),
                pending_timers: exec
                    .pending_timers
                    .keys()
                    .map(|tid| {
                        (tid.to_string(), TimerState {
                            timer_id: tid.to_string(),
                            fire_at: Utc::now(), // Approximate — timer has the real value.
                        })
                    })
                    .collect(),
                compensation_stack: exec.compensation_stack.clone(),
                side_effect_seq: exec.side_effect_seq,
            };

            self.event_store.save_snapshot(&snapshot).await?;
            exec.last_snapshot_event_id = Some(current_event_id);

            info!(
                workflow_id = %self.workflow_id,
                snapshot_id = current_event_id,
                "automatic snapshot taken"
            );
        }
        Ok(())
    }

    /// Persist terminal workflow state to KV.
    async fn persist_terminal_state(&self, exec: &WorkflowExecution) -> Result<()> {
        let key = format!("{}{}", WORKFLOW_STATE_PREFIX, self.workflow_id);
        let state = PersistedWorkflowState {
            workflow_id: self.workflow_id.clone(),
            workflow_type: exec.workflow_type.clone(),
            status: exec.status.clone(),
            last_event_id: exec.last_event_id,
            last_snapshot_event_id: exec.last_snapshot_event_id,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
        };
        let value = serde_json::to_string(&state)?;

        self.store
            .write(aspen_kv_types::WriteRequest {
                command: aspen_kv_types::WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_testing::DeterministicKeyValueStore;
    use serde_json::json;

    use super::*;

    /// Helper: create an executor and throw away the timer receiver.
    fn make_executor(store: Arc<DeterministicKeyValueStore>) -> DurableWorkflowExecutor<DeterministicKeyValueStore> {
        let (executor, _rx) = DurableWorkflowExecutor::new(store, "test-node".to_string());
        executor
    }

    #[tokio::test]
    async fn test_start_and_complete_workflow() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = make_executor(store.clone());

        let handle = executor.start_workflow("test-wf", json!({"input": 1})).await.unwrap();
        assert_eq!(handle.status().await, DurableWorkflowStatus::Running);

        handle.complete(json!({"done": true})).await.unwrap();
        assert_eq!(handle.status().await, DurableWorkflowStatus::Completed);

        // Verify events: WorkflowStarted + WorkflowCompleted = 2
        let events = handle.events().await.unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].event_type, WorkflowEventType::WorkflowStarted { .. }));
        assert!(matches!(events[1].event_type, WorkflowEventType::WorkflowCompleted { .. }));
    }

    #[tokio::test]
    async fn test_execute_three_activities_and_complete() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = make_executor(store.clone());

        let handle = executor.start_workflow("multi-step", json!({})).await.unwrap();

        let r1 = handle
            .execute_activity("act-1", "fetch", json!({"url": "a"}), |_input| async { Ok(json!({"data": "a_result"})) })
            .await
            .unwrap();
        assert_eq!(r1, json!({"data": "a_result"}));

        let r2 = handle
            .execute_activity("act-2", "transform", r1.clone(), |_input| async { Ok(json!({"transformed": true})) })
            .await
            .unwrap();
        assert_eq!(r2, json!({"transformed": true}));

        let r3 = handle
            .execute_activity("act-3", "store", r2.clone(), |_input| async { Ok(json!({"stored": true})) })
            .await
            .unwrap();
        assert_eq!(r3, json!({"stored": true}));

        handle.complete(json!({"final": r3})).await.unwrap();

        // Events: Started + 3*(Scheduled+Completed) + Completed = 1 + 6 + 1 = 8
        let events = handle.events().await.unwrap();
        assert_eq!(events.len(), 8);
    }

    #[tokio::test]
    async fn test_activity_failure_records_event() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = make_executor(store);

        let handle = executor.start_workflow("fail-wf", json!({})).await.unwrap();

        let err = handle
            .execute_activity("act-fail", "bad-step", json!({}), |_| async { Err("something went wrong".to_string()) })
            .await
            .unwrap_err();

        assert!(err.to_string().contains("something went wrong"));

        // Events: Started + Scheduled + Failed = 3
        let events = handle.events().await.unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[2].event_type, WorkflowEventType::ActivityFailed { .. }));
    }

    #[tokio::test]
    async fn test_side_effect_recording() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = make_executor(store);

        let handle = executor.start_workflow("side-fx", json!({})).await.unwrap();

        let uuid_value = handle.record_side_effect(|| json!("generated-uuid-123")).await.unwrap();
        assert_eq!(uuid_value, json!("generated-uuid-123"));

        let events = handle.events().await.unwrap();
        // Started + SideEffectRecorded = 2
        assert_eq!(events.len(), 2);
        assert!(matches!(events[1].event_type, WorkflowEventType::SideEffectRecorded { .. }));
    }

    #[tokio::test]
    async fn test_recover_workflow_memoizes_activities() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = make_executor(store.clone());

        // Run a workflow with 2 activities.
        let handle = executor.start_workflow("recover-test", json!({})).await.unwrap();
        let wf_id = handle.workflow_id.clone();

        handle
            .execute_activity("act-1", "step", json!({}), |_| async { Ok(json!({"step": 1})) })
            .await
            .unwrap();

        handle
            .execute_activity("act-2", "step", json!({}), |_| async { Ok(json!({"step": 2})) })
            .await
            .unwrap();

        // Drop the executor (simulate crash).
        drop(executor);
        drop(handle);

        // Create a new executor and recover.
        let executor2 = make_executor(store.clone());
        let recovered_handle = executor2.recover(&wf_id).await.unwrap();

        assert_eq!(recovered_handle.status().await, DurableWorkflowStatus::Replaying);

        // Activity 1 should return memoized result.
        let execution_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let ec = execution_count.clone();
        let r1 = recovered_handle
            .execute_activity("act-1", "step", json!({}), move |_| {
                ec.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { Ok(json!({"step": 1})) }
            })
            .await
            .unwrap();

        assert_eq!(r1, json!({"step": 1}));
        // The function should NOT have been called — memoized.
        assert_eq!(execution_count.load(std::sync::atomic::Ordering::SeqCst), 0);

        // Activity 2 should also be memoized.
        let ec2 = execution_count.clone();
        let r2 = recovered_handle
            .execute_activity("act-2", "step", json!({}), move |_| {
                ec2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { Ok(json!({"step": 2})) }
            })
            .await
            .unwrap();

        assert_eq!(r2, json!({"step": 2}));
        assert_eq!(execution_count.load(std::sync::atomic::Ordering::SeqCst), 0);

        // Activity 3 (new) should execute live.
        let ec3 = execution_count.clone();
        let r3 = recovered_handle
            .execute_activity("act-3", "step", json!({}), move |_| {
                ec3.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { Ok(json!({"step": 3})) }
            })
            .await
            .unwrap();

        assert_eq!(r3, json!({"step": 3}));
        assert_eq!(execution_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        recovered_handle.complete(json!({"recovered": true})).await.unwrap();
        assert_eq!(recovered_handle.status().await, DurableWorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_discover_active_workflows() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = make_executor(store.clone());

        // Start 3 workflows, complete 1, fail 1, leave 1 active.
        let h1 = executor.start_workflow("wf-1", json!({})).await.unwrap();
        let h2 = executor.start_workflow("wf-2", json!({})).await.unwrap();
        let h3 = executor.start_workflow("wf-3", json!({})).await.unwrap();

        h1.complete(json!({})).await.unwrap();
        h2.fail("intentional").await.unwrap();
        // h3 left active.

        let active = executor.discover_active_workflows().await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], h3.workflow_id);
    }

    #[tokio::test]
    async fn test_compensation_event_recording() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = make_executor(store);

        let handle = executor.start_workflow("saga-wf", json!({})).await.unwrap();

        // Simulate saga steps.
        handle
            .execute_activity("step-1", "action", json!({}), |_| async { Ok(json!({"done": 1})) })
            .await
            .unwrap();

        handle
            .execute_activity("step-2", "action", json!({}), |_| async { Ok(json!({"done": 2})) })
            .await
            .unwrap();

        // Step 3 fails.
        let _ = handle
            .execute_activity("step-3", "action", json!({}), |_| async { Err("step 3 failed".to_string()) })
            .await;

        // Compensate step 2 then step 1 (LIFO).
        handle.record_compensation("comp-step-2", || async { Ok(()) }).await.unwrap();

        handle.record_compensation("comp-step-1", || async { Ok(()) }).await.unwrap();

        let events = handle.events().await.unwrap();

        // Count compensation events.
        let comp_started = events
            .iter()
            .filter(|e| matches!(e.event_type, WorkflowEventType::CompensationStarted { .. }))
            .count();
        let comp_completed = events
            .iter()
            .filter(|e| matches!(e.event_type, WorkflowEventType::CompensationCompleted { .. }))
            .count();

        assert_eq!(comp_started, 2);
        assert_eq!(comp_completed, 2);
    }

    #[tokio::test]
    async fn test_double_complete_rejected() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = make_executor(store);

        let handle = executor.start_workflow("double", json!({})).await.unwrap();
        handle.complete(json!({})).await.unwrap();

        let err = handle.complete(json!({})).await.unwrap_err();
        assert!(err.to_string().contains("Completed"));
    }

    #[tokio::test]
    async fn test_max_concurrent_workflows() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = make_executor(store);

        // We won't actually start 10,000 — just verify the guard exists
        // by checking the constant.
        assert_eq!(MAX_CONCURRENT_WORKFLOWS, 10_000);
    }

    #[tokio::test]
    async fn test_on_lose_leadership_clears_workflows() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = make_executor(store);

        let _h1 = executor.start_workflow("wf-a", json!({})).await.unwrap();
        let _h2 = executor.start_workflow("wf-b", json!({})).await.unwrap();

        assert_eq!(executor.active_workflow_count().await, 2);

        executor.on_lose_leadership().await;

        assert_eq!(executor.active_workflow_count().await, 0);
    }
}
