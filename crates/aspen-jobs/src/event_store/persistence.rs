//! Event and snapshot persistence operations.

use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::WorkflowEvent;
use super::WorkflowEventStore;
use super::event_types::WorkflowEventType;
use super::snapshot::WorkflowSnapshot;
use super::types::EVENT_KEY_PREFIX;
use super::types::MAX_EVENTS_BEFORE_SNAPSHOT;
use super::types::MAX_EVENTS_PER_WORKFLOW;
use super::types::SNAPSHOT_KEY_PREFIX;
use super::types::WorkflowExecutionId;
use crate::error::JobError;
use crate::error::Result;

impl<S: aspen_traits::KeyValueStore + ?Sized + 'static> WorkflowEventStore<S> {
    /// Append an event to the workflow history.
    ///
    /// Events are append-only and immutable once written.
    /// Returns the event ID of the appended event.
    pub async fn append_event(
        &self,
        workflow_id: &WorkflowExecutionId,
        event_type: WorkflowEventType,
        prev_event_id: Option<u64>,
    ) -> Result<u64> {
        let event_id = prev_event_id.map(|id| id + 1).unwrap_or(0);

        // Check workflow event limit
        if event_id >= MAX_EVENTS_PER_WORKFLOW {
            return Err(JobError::ExecutionFailed {
                reason: format!(
                    "Workflow {} exceeded max event limit ({} events). Use continue-as-new.",
                    workflow_id, MAX_EVENTS_PER_WORKFLOW
                ),
            });
        }

        let event = WorkflowEvent::new(event_id, workflow_id.clone(), event_type, &self.hlc, prev_event_id);

        let key = event.storage_key();
        let value = serde_json::to_string(&event).map_err(|e| JobError::SerializationError { source: e })?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        debug!(
            workflow_id = %workflow_id,
            event_id,
            event_type = ?std::mem::discriminant(&event.event_type),
            "event appended"
        );

        // Log warning if approaching snapshot threshold
        if event_id > 0 && event_id.is_multiple_of(MAX_EVENTS_BEFORE_SNAPSHOT) {
            warn!(
                workflow_id = %workflow_id,
                event_id,
                "workflow has {} events - consider taking a snapshot",
                event_id
            );
        }

        Ok(event_id)
    }

    /// Save a workflow snapshot.
    pub async fn save_snapshot(&self, snapshot: &WorkflowSnapshot) -> Result<()> {
        let key = snapshot.storage_key();
        let value = serde_json::to_string(snapshot).map_err(|e| JobError::SerializationError { source: e })?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        // Also append a SnapshotTaken event
        self.append_event(
            &snapshot.workflow_id,
            WorkflowEventType::SnapshotTaken {
                snapshot_id: snapshot.snapshot_id,
                at_event_id: snapshot.at_event_id,
            },
            Some(snapshot.at_event_id),
        )
        .await?;

        info!(
            workflow_id = %snapshot.workflow_id,
            snapshot_id = snapshot.snapshot_id,
            at_event_id = snapshot.at_event_id,
            "snapshot saved"
        );

        Ok(())
    }

    /// Load the latest snapshot for a workflow.
    pub async fn load_latest_snapshot(&self, workflow_id: &WorkflowExecutionId) -> Result<Option<WorkflowSnapshot>> {
        let prefix = format!("{}{}", SNAPSHOT_KEY_PREFIX, workflow_id);

        let scan_result = self
            .store
            .scan(ScanRequest {
                prefix,
                limit_results: Some(100), // Snapshots should be infrequent
                continuation_token: None,
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        let mut snapshots: Vec<WorkflowSnapshot> =
            scan_result.entries.iter().filter_map(|entry| serde_json::from_str(&entry.value).ok()).collect();

        // Get the latest snapshot by snapshot_id
        snapshots.sort_by_key(|s| s.snapshot_id);

        Ok(snapshots.pop())
    }

    /// Get all workflow executions (for administrative purposes).
    ///
    /// Note: This scans all events, so use sparingly.
    pub async fn list_workflow_executions(&self, limit: u32) -> Result<Vec<WorkflowExecutionId>> {
        let scan_result = self
            .store
            .scan(ScanRequest {
                prefix: EVENT_KEY_PREFIX.to_string(),
                limit_results: Some(limit),
                continuation_token: None,
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        let mut workflow_ids = std::collections::HashSet::new();

        for entry in scan_result.entries {
            // Extract workflow_id from key: __wf_events::{workflow_id}::{event_id}
            if let Some(rest) = entry.key.strip_prefix(EVENT_KEY_PREFIX) {
                if let Some(wf_id) = rest.split("::").next() {
                    workflow_ids.insert(WorkflowExecutionId::from_string(wf_id.to_string()));
                }
            }
        }

        Ok(workflow_ids.into_iter().collect())
    }
}
