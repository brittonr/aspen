//! Event sourcing layer for durable workflow execution.
//!
//! This module provides an append-only event log for workflow history,
//! enabling:
//! - Complete audit trail of all workflow operations
//! - Deterministic replay for crash recovery
//! - Activity result memoization (replay returns cached results)
//! - Snapshot-based optimization for long-running workflows
//!
//! ## Architecture
//!
//! Events are stored in Aspen's KV store with keys:
//! - `__wf_events::{workflow_id}::{event_id:020}` - Individual events
//! - `__wf_snapshots::{workflow_id}::{snapshot_id:020}` - State snapshots
//!
//! The 20-digit zero-padded IDs ensure lexicographic ordering for efficient
//! range scans via `scan()`.
//!
//! ## Tiger Style
//!
//! - `MAX_EVENTS_BEFORE_SNAPSHOT` = 1,000 events before mandatory snapshot
//! - `MAX_EVENTS_PER_WORKFLOW` = 50,000 events (Temporal limit)
//! - `MAX_EVENT_SIZE` = 1 MB (matches `MAX_VALUE_SIZE`)
//! - All operations have explicit bounds to prevent resource exhaustion

pub mod event_types;
mod persistence;
pub mod replay;
pub mod snapshot;
mod tests;
pub mod types;

use std::sync::Arc;

use aspen_hlc::HLC;
use aspen_hlc::SerializableTimestamp;
use aspen_hlc::create_hlc;
use aspen_traits::KeyValueStore;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

pub use self::event_types::WorkflowEventType;
pub use self::replay::WorkflowReplayEngine;
pub use self::snapshot::ActivityState;
pub use self::snapshot::CachedActivityResult;
pub use self::snapshot::CompensationEntry;
pub use self::snapshot::TimerState;
pub use self::snapshot::WorkflowSnapshot;
use self::types::EVENT_KEY_PREFIX;
use self::types::EVENT_SCHEMA_VERSION;
pub use self::types::WorkflowExecutionId;

/// Upcast an event from an older schema version to the current version.
///
/// This function implements the upcaster pattern from event sourcing:
/// when reading events from storage, older events may have different schemas.
/// Upcasting transforms them to the current format on read.
///
/// ## Versioning Strategy
///
/// - Version 1 (current): Initial schema, no migration needed
/// - Future versions: Add match arms to migrate from older versions
///
/// ## Tiger Style
///
/// - Each version migration is explicit and auditable
/// - Unknown versions are rejected rather than silently corrupted
/// - Migrations are deterministic and side-effect free
fn upcast_event(event: WorkflowEvent) -> std::result::Result<WorkflowEvent, String> {
    match event.schema_version {
        // Current version - no migration needed
        1 => Ok(event),

        // Version 0 was never released, but handle it for safety
        0 => {
            // If we ever had a v0 schema, migration logic would go here.
            // For now, reject as unknown since v1 is the first release.
            Err(format!(
                "unknown event schema version 0 for event_id {}: v1 is the first supported version",
                event.event_id
            ))
        }

        // Future versions would be handled here during downgrades (if supported)
        v if v > EVENT_SCHEMA_VERSION => Err(format!(
            "event schema version {} is newer than supported version {}: upgrade aspen-jobs",
            v, EVENT_SCHEMA_VERSION
        )),

        // Any other version is unknown
        v => Err(format!("unknown event schema version {} for event_id {}", v, event.event_id)),
    }
}

/// Immutable workflow event for event sourcing.
///
/// Each event captures a discrete state change in the workflow execution.
/// Events are append-only and immutable once written.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Sequential event ID within this workflow execution.
    pub event_id: u64,
    /// Workflow execution ID.
    pub workflow_id: WorkflowExecutionId,
    /// HLC timestamp for causal ordering across nodes.
    pub hlc_timestamp: SerializableTimestamp,
    /// Wall-clock time for human readability.
    pub timestamp: DateTime<Utc>,
    /// Event type and payload.
    pub event_type: WorkflowEventType,
    /// Previous event ID for chain validation (None for first event).
    pub prev_event_id: Option<u64>,
}

impl WorkflowEvent {
    /// Create a new workflow event.
    pub fn new(
        event_id: u64,
        workflow_id: WorkflowExecutionId,
        event_type: WorkflowEventType,
        hlc: &HLC,
        prev_event_id: Option<u64>,
    ) -> Self {
        Self {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id,
            workflow_id,
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
            timestamp: Utc::now(),
            event_type,
            prev_event_id,
        }
    }

    /// Get storage key for this event.
    pub fn storage_key(&self) -> String {
        format!("{}{}::{:020}", EVENT_KEY_PREFIX, self.workflow_id, self.event_id)
    }
}

/// Event store for durable workflow execution.
///
/// Provides append-only event storage with snapshot support,
/// enabling deterministic replay and crash recovery.
pub struct WorkflowEventStore<S: KeyValueStore + ?Sized> {
    /// KV store for persistence.
    store: Arc<S>,
    /// HLC for causal timestamps.
    hlc: Arc<HLC>,
    /// Node ID for logging (reserved for future tracing).
    #[allow(dead_code)]
    node_id: String,
}

impl<S: KeyValueStore + ?Sized + 'static> WorkflowEventStore<S> {
    /// Create a new event store.
    pub fn new(store: Arc<S>, node_id: String) -> Self {
        let hlc = create_hlc(&node_id);
        Self {
            store,
            hlc: Arc::new(hlc),
            node_id,
        }
    }

    /// Get reference to the HLC.
    pub fn hlc(&self) -> &Arc<HLC> {
        &self.hlc
    }
}
