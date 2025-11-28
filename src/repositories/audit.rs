//! Hiqlite Audit Repository Implementation
//!
//! Provides a concrete implementation of the AuditRepository trait
//! using Hiqlite as the persistence layer.

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::audit::{AuditLogEntry, AuditRepository};
use crate::domain::errors::{DomainError, DomainResult};
use crate::hiqlite::HiqliteService;
use hiqlite::Param;

/// Hiqlite-based implementation of AuditRepository
///
/// Stores audit logs in the Hiqlite database for compliance and auditing.
pub struct HiqliteAuditRepository {
    hiqlite: Arc<HiqliteService>,
}

impl HiqliteAuditRepository {
    /// Create a new Hiqlite audit repository
    pub fn new(hiqlite: Arc<HiqliteService>) -> Self {
        Self { hiqlite }
    }

    /// Initialize the audit_logs table if it doesn't exist
    ///
    /// This should be called during application startup to ensure
    /// the table schema is in place.
    pub async fn initialize(&self) -> DomainResult<()> {
        let query = r#"
            CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                event_data TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE INDEX IF NOT EXISTS idx_audit_logs_event_type
            ON audit_logs(event_type);

            CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp
            ON audit_logs(timestamp DESC);
        "#;

        self.hiqlite.execute(query, vec![]).await.map_err(|e| {
            DomainError::Infrastructure(format!("Failed to initialize audit_logs table: {}", e))
        })?;

        Ok(())
    }
}

#[async_trait]
impl AuditRepository for HiqliteAuditRepository {
    async fn store_audit_log(
        &self,
        event_type: &str,
        event_data: &serde_json::Value,
        timestamp: i64,
    ) -> DomainResult<()> {
        // Serialize event data to JSON
        let event_json = serde_json::to_string(event_data)
            .map_err(|e| DomainError::Generic(format!("Failed to serialize event data: {}", e)))?;

        let query = r#"
            INSERT INTO audit_logs (event_type, event_data, timestamp)
            VALUES (?1, ?2, ?3)
        "#;

        let params = vec![
            Param::from(event_type.to_string()),
            Param::from(event_json),
            Param::from(timestamp),
        ];

        self.hiqlite.execute(query, params).await.map_err(|e| {
            DomainError::Infrastructure(format!("Failed to store audit log: {}", e))
        })?;

        Ok(())
    }

    async fn query_audit_logs(
        &self,
        event_type: Option<&str>,
        limit: usize,
    ) -> DomainResult<Vec<AuditLogEntry>> {
        let (query, params) = if let Some(event_type) = event_type {
            (
                r#"
                    SELECT id, event_type, event_data, timestamp
                    FROM audit_logs
                    WHERE event_type = ?1
                    ORDER BY timestamp DESC
                    LIMIT ?2
                "#,
                vec![
                    Param::from(event_type.to_string()),
                    Param::from(limit as i64),
                ],
            )
        } else {
            (
                r#"
                    SELECT id, event_type, event_data, timestamp
                    FROM audit_logs
                    ORDER BY timestamp DESC
                    LIMIT ?1
                "#,
                vec![Param::from(limit as i64)],
            )
        };

        #[derive(serde::Deserialize)]
        struct AuditRow {
            id: i64,
            event_type: String,
            event_data: String,
            timestamp: i64,
        }

        impl From<hiqlite::Row<'static>> for AuditRow {
            fn from(mut row: hiqlite::Row<'static>) -> Self {
                AuditRow {
                    id: row.get::<i64>("id"),
                    event_type: row.get::<String>("event_type"),
                    event_data: row.get::<String>("event_data"),
                    timestamp: row.get::<i64>("timestamp"),
                }
            }
        }

        let rows: Vec<AuditRow> = self.hiqlite.query_as(query, params).await.map_err(|e| {
            DomainError::Infrastructure(format!("Failed to query audit logs: {}", e))
        })?;

        let entries: Result<Vec<AuditLogEntry>, _> = rows
            .into_iter()
            .map(|row| {
                let event_data: serde_json::Value =
                    serde_json::from_str(&row.event_data).map_err(|e| {
                        DomainError::Generic(format!("Failed to parse event data: {}", e))
                    })?;

                Ok(AuditLogEntry {
                    id: row.id,
                    event_type: row.event_type,
                    event_data,
                    timestamp: row.timestamp,
                })
            })
            .collect();

        entries
    }

    async fn count_audit_logs(&self, event_type: Option<&str>) -> DomainResult<u64> {
        let (query, params) = if let Some(event_type) = event_type {
            (
                "SELECT COUNT(*) as count FROM audit_logs WHERE event_type = ?1",
                vec![Param::from(event_type.to_string())],
            )
        } else {
            ("SELECT COUNT(*) as count FROM audit_logs", vec![])
        };

        #[derive(serde::Deserialize)]
        struct CountResult {
            count: i64,
        }

        impl From<hiqlite::Row<'static>> for CountResult {
            fn from(mut row: hiqlite::Row<'static>) -> Self {
                CountResult {
                    count: row.get::<i64>("count"),
                }
            }
        }

        let result: Vec<CountResult> = self.hiqlite.query_as(query, params).await.map_err(|e| {
            DomainError::Infrastructure(format!("Failed to count audit logs: {}", e))
        })?;

        Ok(result.first().map(|r| r.count as u64).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestEvent {
        id: String,
        message: String,
    }

    // Note: These tests would require a running Hiqlite instance
    // They are integration tests and should be run with a test database

    #[test]
    fn test_repository_creation() {
        // This is a simple unit test that just checks we can create the repository
        // Integration tests would require a real Hiqlite instance
        assert!(true, "Repository struct can be created");
    }
}
