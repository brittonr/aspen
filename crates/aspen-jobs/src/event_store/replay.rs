//! Event replay and memoization for deterministic workflow execution.

use std::collections::HashMap;

use aspen_kv_types::ScanRequest;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::WorkflowEvent;
use super::WorkflowEventStore;
use super::event_types::WorkflowEventType;
use super::snapshot::CachedActivityResult;
use super::types::EVENT_KEY_PREFIX;
use super::types::EVENT_SCHEMA_VERSION;
use super::types::MAX_EVENTS_PER_SCAN;
use super::types::WorkflowExecutionId;
use super::upcast_event;
use crate::error::JobError;
use crate::error::Result;

impl<S: aspen_traits::KeyValueStore + ?Sized + 'static> WorkflowEventStore<S> {
    /// Replay events from a starting point.
    ///
    /// Returns events in order, starting from `from_event_id`.
    /// If `from_event_id` is None, starts from the beginning.
    pub async fn replay_events(
        &self,
        workflow_id: &WorkflowExecutionId,
        from_event_id: Option<u64>,
    ) -> Result<Vec<WorkflowEvent>> {
        let prefix = format!("{}{}", EVENT_KEY_PREFIX, workflow_id);
        let start_suffix = from_event_id.map(|id| format!("::{:020}", id));

        // Build scan prefix
        let scan_prefix = if let Some(ref suffix) = start_suffix {
            format!("{}{}", prefix, suffix)
        } else {
            prefix.clone()
        };

        let scan_result = self
            .store
            .scan(ScanRequest {
                prefix: scan_prefix,
                limit: Some(MAX_EVENTS_PER_SCAN),
                continuation_token: None,
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        let mut events = Vec::with_capacity(scan_result.entries.len());

        for entry in scan_result.entries {
            match serde_json::from_str::<WorkflowEvent>(&entry.value) {
                Ok(event) => {
                    // Handle schema version migration if needed
                    if event.schema_version != EVENT_SCHEMA_VERSION {
                        debug!(
                            workflow_id = %workflow_id,
                            event_id = event.event_id,
                            schema_version = event.schema_version,
                            current_version = EVENT_SCHEMA_VERSION,
                            "migrating event from different schema version"
                        );
                    }

                    // Upcast event to current schema version
                    match upcast_event(event) {
                        Ok(migrated_event) => {
                            events.push(migrated_event);
                        }
                        Err(e) => {
                            warn!(
                                key = %entry.key,
                                error = %e,
                                "failed to upcast event, skipping"
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        key = %entry.key,
                        error = %e,
                        "failed to deserialize event, skipping"
                    );
                }
            }
        }

        // Sort by event_id to ensure ordering
        events.sort_by_key(|e| e.event_id);

        info!(
            workflow_id = %workflow_id,
            from_event_id = ?from_event_id,
            event_count = events.len(),
            "replayed events"
        );

        Ok(events)
    }

    /// Get the latest event ID for a workflow.
    pub async fn get_latest_event_id(&self, workflow_id: &WorkflowExecutionId) -> Result<Option<u64>> {
        let events = self.replay_events(workflow_id, None).await?;
        Ok(events.last().map(|e| e.event_id))
    }

    /// Check if a workflow should take a snapshot.
    pub fn should_snapshot(&self, current_event_id: u64, last_snapshot_event_id: Option<u64>) -> bool {
        let events_since_snapshot = match last_snapshot_event_id {
            Some(snap_id) => current_event_id.saturating_sub(snap_id),
            None => current_event_id,
        };
        events_since_snapshot >= super::types::MAX_EVENTS_BEFORE_SNAPSHOT
    }

    /// Get cached activity result from event history.
    ///
    /// This implements activity memoization - during replay,
    /// completed activities return their cached result instead
    /// of re-executing.
    pub async fn get_cached_activity_result(
        &self,
        workflow_id: &WorkflowExecutionId,
        activity_id: &str,
    ) -> Result<Option<CachedActivityResult>> {
        let events = self.replay_events(workflow_id, None).await?;

        // Find ActivityCompleted event for this activity
        for event in events.iter().rev() {
            if let WorkflowEventType::ActivityCompleted {
                activity_id: ref aid,
                ref result,
                duration_ms,
            } = event.event_type
            {
                if aid == activity_id {
                    return Ok(Some(CachedActivityResult {
                        activity_id: activity_id.to_string(),
                        result: result.clone(),
                        cached_at: event.timestamp,
                        duration_ms,
                    }));
                }
            }
        }

        Ok(None)
    }
}

/// Replay engine for deterministic workflow execution.
///
/// This engine replays events and returns memoized results
/// for activities, timers, and side effects, ensuring
/// deterministic execution during recovery.
pub struct WorkflowReplayEngine {
    /// Events being replayed.
    events: Vec<WorkflowEvent>,
    /// Current replay position (bounded by event count).
    replay_index: u32,
    /// Whether we're in replay mode.
    is_replaying: bool,
    /// Cached activity results (activity_id -> result).
    activity_cache: HashMap<String, serde_json::Value>,
    /// Cached side effects (seq -> value).
    side_effect_cache: HashMap<u64, serde_json::Value>,
    /// Side effect sequence counter.
    side_effect_seq: u64,
}

impl WorkflowReplayEngine {
    /// Create a new replay engine from event history.
    pub fn new(events: Vec<WorkflowEvent>) -> Self {
        // Pre-populate caches from events
        let mut activity_cache = HashMap::new();
        let mut side_effect_cache = HashMap::new();

        for event in &events {
            match &event.event_type {
                WorkflowEventType::ActivityCompleted {
                    activity_id, result, ..
                } => {
                    activity_cache.insert(activity_id.clone(), result.clone());
                }
                WorkflowEventType::SideEffectRecorded { seq, value } => {
                    side_effect_cache.insert(*seq, value.clone());
                }
                _ => {}
            }
        }

        Self {
            events,
            replay_index: 0,
            is_replaying: true,
            activity_cache,
            side_effect_cache,
            side_effect_seq: 0,
        }
    }

    /// Check if we're still replaying.
    pub fn is_replaying(&self) -> bool {
        self.is_replaying && (self.replay_index as usize) < self.events.len()
    }

    /// Get cached activity result during replay.
    pub fn get_activity_result(&self, activity_id: &str) -> Option<&serde_json::Value> {
        if self.is_replaying {
            self.activity_cache.get(activity_id)
        } else {
            None
        }
    }

    /// Get cached side effect during replay.
    pub fn get_side_effect(&mut self) -> Option<serde_json::Value> {
        if self.is_replaying {
            let seq = self.side_effect_seq;
            self.side_effect_seq += 1;
            self.side_effect_cache.get(&seq).cloned()
        } else {
            None
        }
    }

    /// Record a new side effect (during non-replay execution).
    pub fn record_side_effect(&mut self, value: serde_json::Value) -> u64 {
        let seq = self.side_effect_seq;
        self.side_effect_seq += 1;
        self.side_effect_cache.insert(seq, value);
        seq
    }

    /// Advance replay position.
    pub fn advance(&mut self) {
        if (self.replay_index as usize) < self.events.len() {
            self.replay_index += 1;
        }
        if (self.replay_index as usize) >= self.events.len() {
            self.is_replaying = false;
        }
    }

    /// Get current event during replay.
    pub fn current_event(&self) -> Option<&WorkflowEvent> {
        self.events.get(self.replay_index as usize)
    }

    /// Mark replay as complete.
    pub fn finish_replay(&mut self) {
        self.is_replaying = false;
    }
}
