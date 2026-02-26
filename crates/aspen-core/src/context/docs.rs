//! Document synchronization types.
//!
//! Provides traits and types for CRDT-based document synchronization
//! using iroh-docs.

use async_trait::async_trait;

/// Docs ticket for connecting to a peer cluster.
#[derive(Debug, Clone)]
pub struct AspenDocsTicket {
    /// Cluster ID of the peer.
    pub cluster_id: String,
    /// Priority for conflict resolution.
    pub priority: u8,
}

/// Document synchronization provider.
#[async_trait]
pub trait DocsSyncProvider: Send + Sync {
    /// Join a document for synchronization.
    async fn join_document(&self, doc_id: &[u8]) -> Result<(), String>;

    /// Leave a document synchronization.
    async fn leave_document(&self, doc_id: &[u8]) -> Result<(), String>;

    /// Get document content.
    async fn get_document(&self, doc_id: &[u8]) -> Result<Vec<u8>, String>;

    /// Set an entry in the docs namespace.
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), String>;

    /// Get an entry from the docs namespace.
    async fn get_entry(&self, key: &[u8]) -> Result<Option<(Vec<u8>, u64, String)>, String>; // (value, size, hash)

    /// Delete an entry from the docs namespace.
    async fn delete_entry(&self, key: Vec<u8>) -> Result<(), String>;

    /// List entries with optional prefix filter.
    async fn list_entries(&self, prefix: Option<String>, limit: Option<u32>) -> Result<Vec<DocsEntry>, String>;

    /// Get docs namespace status.
    async fn get_status(&self) -> Result<DocsStatus, String>;

    /// Get namespace ID as string.
    fn namespace_id(&self) -> String;

    /// Get author ID as string.
    fn author_id(&self) -> String;
}

/// Entry in docs namespace.
#[derive(Debug, Clone)]
pub struct DocsEntry {
    pub key: String,
    pub size_bytes: u64,
    pub hash: String,
}

/// Status of docs namespace.
#[derive(Debug, Clone)]
pub struct DocsStatus {
    pub is_enabled: bool,
    pub namespace_id: Option<String>,
    pub author_id: Option<String>,
    pub entry_count: Option<u64>,
    pub replica_open: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // AspenDocsTicket Tests
    // ========================================================================

    #[test]
    fn aspen_docs_ticket_construction() {
        let ticket = AspenDocsTicket {
            cluster_id: "cluster-abc".to_string(),
            priority: 10,
        };
        assert_eq!(ticket.cluster_id, "cluster-abc");
        assert_eq!(ticket.priority, 10);
    }

    #[test]
    fn aspen_docs_ticket_debug() {
        let ticket = AspenDocsTicket {
            cluster_id: "debug-cluster".to_string(),
            priority: 5,
        };
        let debug = format!("{:?}", ticket);
        assert!(debug.contains("AspenDocsTicket"));
        assert!(debug.contains("debug-cluster"));
        assert!(debug.contains("5"));
    }

    #[test]
    fn aspen_docs_ticket_clone() {
        let ticket = AspenDocsTicket {
            cluster_id: "original".to_string(),
            priority: 255,
        };
        let cloned = ticket.clone();
        assert_eq!(ticket.cluster_id, cloned.cluster_id);
        assert_eq!(ticket.priority, cloned.priority);
    }

    #[test]
    fn aspen_docs_ticket_priority_range() {
        // u8 has range 0-255
        let min = AspenDocsTicket {
            cluster_id: "min".to_string(),
            priority: 0,
        };
        let max = AspenDocsTicket {
            cluster_id: "max".to_string(),
            priority: 255,
        };
        assert_eq!(min.priority, 0);
        assert_eq!(max.priority, 255);
    }

    // ========================================================================
    // DocsEntry Tests
    // ========================================================================

    #[test]
    fn docs_entry_construction() {
        let entry = DocsEntry {
            key: "users/alice".to_string(),
            size_bytes: 1024,
            hash: "abc123def456".to_string(),
        };
        assert_eq!(entry.key, "users/alice");
        assert_eq!(entry.size_bytes, 1024);
        assert_eq!(entry.hash, "abc123def456");
    }

    #[test]
    fn docs_entry_debug() {
        let entry = DocsEntry {
            key: "test-key".to_string(),
            size_bytes: 500,
            hash: "hash123".to_string(),
        };
        let debug = format!("{:?}", entry);
        assert!(debug.contains("DocsEntry"));
        assert!(debug.contains("test-key"));
        assert!(debug.contains("500"));
        assert!(debug.contains("hash123"));
    }

    #[test]
    fn docs_entry_clone() {
        let entry = DocsEntry {
            key: "clone-test".to_string(),
            size_bytes: 2048,
            hash: "original-hash".to_string(),
        };
        let cloned = entry.clone();
        assert_eq!(entry.key, cloned.key);
        assert_eq!(entry.size_bytes, cloned.size_bytes);
        assert_eq!(entry.hash, cloned.hash);
    }

    #[test]
    fn docs_entry_zero_size() {
        let entry = DocsEntry {
            key: "empty".to_string(),
            size_bytes: 0,
            hash: "empty-hash".to_string(),
        };
        assert_eq!(entry.size_bytes, 0);
    }

    #[test]
    fn docs_entry_large_size() {
        let entry = DocsEntry {
            key: "large".to_string(),
            size_bytes: u64::MAX,
            hash: "large-hash".to_string(),
        };
        assert_eq!(entry.size_bytes, u64::MAX);
    }

    // ========================================================================
    // DocsStatus Tests
    // ========================================================================

    #[test]
    fn docs_status_enabled() {
        let status = DocsStatus {
            is_enabled: true,
            namespace_id: Some("ns-123".to_string()),
            author_id: Some("author-456".to_string()),
            entry_count: Some(100),
            replica_open: Some(true),
        };
        assert!(status.is_enabled);
        assert_eq!(status.namespace_id, Some("ns-123".to_string()));
        assert_eq!(status.author_id, Some("author-456".to_string()));
        assert_eq!(status.entry_count, Some(100));
        assert_eq!(status.replica_open, Some(true));
    }

    #[test]
    fn docs_status_disabled() {
        let status = DocsStatus {
            is_enabled: false,
            namespace_id: None,
            author_id: None,
            entry_count: None,
            replica_open: None,
        };
        assert!(!status.is_enabled);
        assert!(status.namespace_id.is_none());
        assert!(status.author_id.is_none());
        assert!(status.entry_count.is_none());
        assert!(status.replica_open.is_none());
    }

    #[test]
    fn docs_status_debug() {
        let status = DocsStatus {
            is_enabled: true,
            namespace_id: Some("debug-ns".to_string()),
            author_id: None,
            entry_count: Some(50),
            replica_open: Some(false),
        };
        let debug = format!("{:?}", status);
        assert!(debug.contains("DocsStatus"));
        assert!(debug.contains("is_enabled: true"));
        assert!(debug.contains("debug-ns"));
    }

    #[test]
    fn docs_status_clone() {
        let status = DocsStatus {
            is_enabled: true,
            namespace_id: Some("clone-ns".to_string()),
            author_id: Some("clone-author".to_string()),
            entry_count: Some(25),
            replica_open: Some(true),
        };
        let cloned = status.clone();
        assert_eq!(status.is_enabled, cloned.is_enabled);
        assert_eq!(status.namespace_id, cloned.namespace_id);
        assert_eq!(status.author_id, cloned.author_id);
        assert_eq!(status.entry_count, cloned.entry_count);
        assert_eq!(status.replica_open, cloned.replica_open);
    }

    #[test]
    fn docs_status_partial_fields() {
        // Mix of Some and None fields
        let status = DocsStatus {
            is_enabled: true,
            namespace_id: Some("partial-ns".to_string()),
            author_id: None,
            entry_count: Some(10),
            replica_open: None,
        };
        assert!(status.is_enabled);
        assert!(status.namespace_id.is_some());
        assert!(status.author_id.is_none());
        assert!(status.entry_count.is_some());
        assert!(status.replica_open.is_none());
    }
}
