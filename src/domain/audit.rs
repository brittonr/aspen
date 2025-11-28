//! Audit Repository Trait
//!
//! This module defines the domain interface for audit logging.
//! Infrastructure provides concrete implementations (e.g., HiqliteAuditRepository).
//!
//! This follows the Dependency Inversion Principle - the domain defines
//! the interface it needs, and infrastructure adapts to it.

use async_trait::async_trait;
use serde::Serialize;

use super::errors::DomainResult;

/// Repository for storing audit logs
///
/// Domain services and event handlers use this trait to persist audit logs
/// without depending on specific database implementations.
#[async_trait]
pub trait AuditRepository: Send + Sync {
    /// Store an audit log entry
    ///
    /// # Arguments
    /// * `event_type` - The type of event (e.g., "JobSubmitted", "JobCompleted")
    /// * `event_data` - Event data as JSON value
    /// * `timestamp` - Unix timestamp of the event
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn store_audit_log(
        &self,
        event_type: &str,
        event_data: &serde_json::Value,
        timestamp: i64,
    ) -> DomainResult<()>;

    /// Query audit logs by event type
    ///
    /// # Arguments
    /// * `event_type` - Filter by event type (None for all events)
    /// * `limit` - Maximum number of entries to return
    ///
    /// # Returns
    /// Vector of audit log entries as JSON values
    async fn query_audit_logs(
        &self,
        event_type: Option<&str>,
        limit: usize,
    ) -> DomainResult<Vec<AuditLogEntry>>;

    /// Count audit logs by event type
    ///
    /// # Arguments
    /// * `event_type` - Filter by event type (None for all events)
    ///
    /// # Returns
    /// Number of matching audit log entries
    async fn count_audit_logs(&self, event_type: Option<&str>) -> DomainResult<u64>;
}

/// Audit log entry returned from queries
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub event_type: String,
    pub event_data: serde_json::Value,
    pub timestamp: i64,
}
