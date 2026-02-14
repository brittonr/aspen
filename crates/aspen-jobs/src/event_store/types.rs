//! Core types for workflow event sourcing.

use serde::Deserialize;
use serde::Serialize;

/// Key prefix for workflow events.
pub(crate) const EVENT_KEY_PREFIX: &str = "__wf_events::";

/// Key prefix for workflow snapshots.
pub(crate) const SNAPSHOT_KEY_PREFIX: &str = "__wf_snapshots::";

/// Maximum events before a snapshot is recommended.
/// Tiger Style: Fixed limit to prevent unbounded replay time.
pub(crate) const MAX_EVENTS_BEFORE_SNAPSHOT: u64 = 1_000;

/// Maximum events per workflow execution (Temporal-style limit).
/// Workflows exceeding this should use continue-as-new.
pub(crate) const MAX_EVENTS_PER_WORKFLOW: u64 = 50_000;

/// Maximum events to scan in a single replay operation.
pub(crate) const MAX_EVENTS_PER_SCAN: u32 = 10_000;

/// Schema version for event format evolution.
/// Increment this when making breaking changes to WorkflowEvent or WorkflowEventType.
pub(crate) const EVENT_SCHEMA_VERSION: u32 = 1;

/// Schema version for snapshot format evolution.
pub(crate) const _SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Unique identifier for a workflow execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowExecutionId(String);

impl WorkflowExecutionId {
    /// Create a new unique workflow execution ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create from an existing string.
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WorkflowExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkflowExecutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
