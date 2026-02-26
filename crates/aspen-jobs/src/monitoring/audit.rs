//! Audit logging types and filters.

use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use crate::job::JobId;

/// Audit log entry for job operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Entry ID.
    pub id: String,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Job ID.
    pub job_id: Option<JobId>,
    /// User/system that triggered the action.
    pub actor: String,
    /// Action performed.
    pub action: AuditAction,
    /// Resource affected.
    pub resource: String,
    /// Old value (for updates).
    pub old_value: Option<String>,
    /// New value (for updates).
    pub new_value: Option<String>,
    /// Result of the action.
    pub result: AuditResult,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

/// Audit action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    /// Job created.
    JobCreated,
    /// Job updated.
    JobUpdated,
    /// Job deleted.
    JobDeleted,
    /// Job started.
    JobStarted,
    /// Job completed.
    JobCompleted,
    /// Job failed.
    JobFailed,
    /// Job retried.
    JobRetried,
    /// Job cancelled.
    JobCancelled,
    /// Worker registered.
    WorkerRegistered,
    /// Worker deregistered.
    WorkerDeregistered,
    /// Configuration changed.
    ConfigChanged,
}

/// Result of an audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditResult {
    /// Action succeeded.
    Success,
    /// Action failed.
    Failure,
    /// Action partially succeeded.
    Partial,
}

/// Filter for querying audit log.
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    /// Filter by action.
    pub action: Option<AuditAction>,
    /// Filter by actor.
    pub actor: Option<String>,
    /// Filter by resource.
    pub resource: Option<String>,
    /// Filter by result.
    pub result: Option<AuditResult>,
    /// Filter by time range.
    pub since: Option<DateTime<Utc>>,
}

impl AuditFilter {
    pub(crate) fn matches(&self, entry: &AuditLogEntry) -> bool {
        if let Some(action) = self.action {
            if entry.action != action {
                return false;
            }
        }

        if let Some(ref actor) = self.actor {
            if &entry.actor != actor {
                return false;
            }
        }

        if let Some(ref resource) = self.resource {
            if &entry.resource != resource {
                return false;
            }
        }

        if let Some(result) = self.result {
            if entry.result != result {
                return false;
            }
        }

        if let Some(since) = self.since {
            if entry.timestamp < since {
                return false;
            }
        }

        true
    }
}
