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
    pub size: u64,
    pub hash: String,
}

/// Status of docs namespace.
#[derive(Debug, Clone)]
pub struct DocsStatus {
    pub enabled: bool,
    pub namespace_id: Option<String>,
    pub author_id: Option<String>,
    pub entry_count: Option<u64>,
    pub replica_open: Option<bool>,
}
