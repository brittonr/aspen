use std::path::{Path, PathBuf};

use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

/// Table definition for node metadata.
/// Key: node_id (u64), Value: serialized NodeMetadata (bincode)
const NODE_METADATA_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("node_metadata");

/// Persistent metadata store for cluster nodes.
///
/// Stores node registration information including endpoint addresses, Raft addresses,
/// status, and timestamps. Backed by redb for ACID persistence.
pub struct MetadataStore {
    db: Database,
    path: PathBuf,
}

/// Metadata for a registered cluster node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeMetadata {
    /// Logical Raft node identifier.
    pub node_id: u64,

    /// Iroh endpoint ID (hex-encoded).
    pub endpoint_id: String,

    /// Raft RPC address.
    pub raft_addr: String,

    /// Current node status.
    pub status: NodeStatus,

    /// Unix timestamp (seconds) when the node was last updated.
    pub last_updated_secs: u64,
}

/// Current status of a cluster node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    /// Node is starting up.
    Starting,
    /// Node is online and healthy.
    Online,
    /// Node is experiencing issues but still reachable.
    Degraded,
    /// Node is offline or unreachable.
    Offline,
}

impl MetadataStore {
    /// Create or open a metadata store at the given path.
    ///
    /// Creates the directory and database file if they don't exist.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, MetadataError> {
        let path = path.as_ref().to_path_buf();

        // Ensure the parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context(CreateDirectorySnafu { path: parent })?;
        }

        let db = Database::create(&path).context(OpenDatabaseSnafu { path: &path })?;

        // Initialize the table
        let write_txn = db.begin_write().context(BeginWriteSnafu)?;
        {
            write_txn
                .open_table(NODE_METADATA_TABLE)
                .context(OpenTableSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(Self { db, path })
    }

    /// Register a new node or update an existing node's metadata.
    pub fn register_node(&self, metadata: NodeMetadata) -> Result<(), MetadataError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn
                .open_table(NODE_METADATA_TABLE)
                .context(OpenTableSnafu)?;

            let serialized = bincode::serialize(&metadata).context(SerializeSnafu)?;
            table
                .insert(metadata.node_id, serialized.as_slice())
                .context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(())
    }

    /// Get metadata for a specific node.
    ///
    /// Returns `None` if the node is not registered.
    pub fn get_node(&self, node_id: u64) -> Result<Option<NodeMetadata>, MetadataError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn
            .open_table(NODE_METADATA_TABLE)
            .context(OpenTableSnafu)?;

        let result = match table.get(node_id).context(GetSnafu)? {
            Some(value) => {
                let bytes = value.value();
                let metadata: NodeMetadata =
                    bincode::deserialize(bytes).context(DeserializeSnafu)?;
                Some(metadata)
            }
            None => None,
        };

        Ok(result)
    }

    /// List all registered nodes.
    pub fn list_nodes(&self) -> Result<Vec<NodeMetadata>, MetadataError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn
            .open_table(NODE_METADATA_TABLE)
            .context(OpenTableSnafu)?;

        let mut nodes = Vec::new();
        let range = table.range::<u64>(..).context(RangeSnafu)?;

        for item in range {
            let (_key, value) = item.context(IteratorSnafu)?;
            let bytes = value.value();
            let metadata: NodeMetadata = bincode::deserialize(bytes).context(DeserializeSnafu)?;
            nodes.push(metadata);
        }

        Ok(nodes)
    }

    /// Update the status of a node.
    ///
    /// Returns an error if the node is not registered.
    pub fn update_status(&self, node_id: u64, status: NodeStatus) -> Result<(), MetadataError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn
                .open_table(NODE_METADATA_TABLE)
                .context(OpenTableSnafu)?;

            // Read existing metadata
            let metadata_bytes = {
                let existing = table
                    .get(node_id)
                    .context(GetSnafu)?
                    .ok_or(MetadataError::NodeNotFound { node_id })?;
                existing.value().to_vec()
            };

            let mut metadata: NodeMetadata =
                bincode::deserialize(&metadata_bytes).context(DeserializeSnafu)?;

            // Update status and timestamp
            metadata.status = status;
            metadata.last_updated_secs = current_timestamp_secs();

            // Write back
            let serialized = bincode::serialize(&metadata).context(SerializeSnafu)?;
            table
                .insert(node_id, serialized.as_slice())
                .context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(())
    }

    /// Remove a node from the metadata store.
    pub fn remove_node(&self, node_id: u64) -> Result<(), MetadataError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn
                .open_table(NODE_METADATA_TABLE)
                .context(OpenTableSnafu)?;
            table.remove(node_id).context(RemoveSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(())
    }

    /// Get the path to the metadata store database file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Get the current Unix timestamp in seconds.
fn current_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_secs()
}

/// Metadata store errors.
#[derive(Debug, Snafu)]
pub enum MetadataError {
    #[snafu(display("failed to create directory {}: {source}", path.display()))]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("failed to open database at {}: {source}", path.display()))]
    OpenDatabase {
        path: PathBuf,
        #[snafu(source(from(redb::DatabaseError, Box::new)))]
        source: Box<redb::DatabaseError>,
    },

    #[snafu(display("failed to begin write transaction: {source}"))]
    BeginWrite {
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    #[snafu(display("failed to begin read transaction: {source}"))]
    BeginRead {
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    #[snafu(display("failed to open table: {source}"))]
    OpenTable {
        #[snafu(source(from(redb::TableError, Box::new)))]
        source: Box<redb::TableError>,
    },

    #[snafu(display("failed to commit transaction: {source}"))]
    Commit {
        #[snafu(source(from(redb::CommitError, Box::new)))]
        source: Box<redb::CommitError>,
    },

    #[snafu(display("failed to insert into table: {source}"))]
    Insert {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to get from table: {source}"))]
    Get {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to remove from table: {source}"))]
    Remove {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to iterate table range: {source}"))]
    Range {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to iterate table: {source}"))]
    Iterator {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to serialize metadata: {source}"))]
    Serialize {
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    #[snafu(display("failed to deserialize metadata: {source}"))]
    Deserialize {
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    #[snafu(display("node {node_id} not found in metadata store"))]
    NodeNotFound { node_id: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_register_and_get_node() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("metadata.redb");
        let store = MetadataStore::new(&db_path).unwrap();

        let metadata = NodeMetadata {
            node_id: 1,
            endpoint_id: "test-endpoint-id".into(),
            raft_addr: "127.0.0.1:26000".into(),
            status: NodeStatus::Online,
            last_updated_secs: current_timestamp_secs(),
        };

        store.register_node(metadata.clone()).unwrap();

        let retrieved = store.get_node(1).unwrap();
        assert_eq!(retrieved, Some(metadata));

        let not_found = store.get_node(999).unwrap();
        assert_eq!(not_found, None);
    }

    #[test]
    fn test_list_nodes() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("metadata.redb");
        let store = MetadataStore::new(&db_path).unwrap();

        let metadata1 = NodeMetadata {
            node_id: 1,
            endpoint_id: "endpoint-1".into(),
            raft_addr: "127.0.0.1:26001".into(),
            status: NodeStatus::Online,
            last_updated_secs: current_timestamp_secs(),
        };

        let metadata2 = NodeMetadata {
            node_id: 2,
            endpoint_id: "endpoint-2".into(),
            raft_addr: "127.0.0.1:26002".into(),
            status: NodeStatus::Starting,
            last_updated_secs: current_timestamp_secs(),
        };

        store.register_node(metadata1.clone()).unwrap();
        store.register_node(metadata2.clone()).unwrap();

        let nodes = store.list_nodes().unwrap();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&metadata1));
        assert!(nodes.contains(&metadata2));
    }

    #[test]
    fn test_update_status() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("metadata.redb");
        let store = MetadataStore::new(&db_path).unwrap();

        let metadata = NodeMetadata {
            node_id: 1,
            endpoint_id: "test-endpoint-id".into(),
            raft_addr: "127.0.0.1:26000".into(),
            status: NodeStatus::Starting,
            last_updated_secs: current_timestamp_secs(),
        };

        store.register_node(metadata).unwrap();

        store.update_status(1, NodeStatus::Online).unwrap();

        let updated = store.get_node(1).unwrap().unwrap();
        assert_eq!(updated.status, NodeStatus::Online);
        assert_eq!(updated.node_id, 1);
    }

    #[test]
    fn test_update_status_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("metadata.redb");
        let store = MetadataStore::new(&db_path).unwrap();

        let result = store.update_status(999, NodeStatus::Online);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MetadataError::NodeNotFound { node_id: 999 }
        ));
    }

    #[test]
    fn test_remove_node() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("metadata.redb");
        let store = MetadataStore::new(&db_path).unwrap();

        let metadata = NodeMetadata {
            node_id: 1,
            endpoint_id: "test-endpoint-id".into(),
            raft_addr: "127.0.0.1:26000".into(),
            status: NodeStatus::Online,
            last_updated_secs: current_timestamp_secs(),
        };

        store.register_node(metadata).unwrap();
        assert!(store.get_node(1).unwrap().is_some());

        store.remove_node(1).unwrap();
        assert!(store.get_node(1).unwrap().is_none());
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("metadata.redb");

        let metadata = NodeMetadata {
            node_id: 1,
            endpoint_id: "test-endpoint-id".into(),
            raft_addr: "127.0.0.1:26000".into(),
            status: NodeStatus::Online,
            last_updated_secs: current_timestamp_secs(),
        };

        // Create store and write data
        {
            let store = MetadataStore::new(&db_path).unwrap();
            store.register_node(metadata.clone()).unwrap();
        }

        // Reopen store and verify data persisted
        {
            let store = MetadataStore::new(&db_path).unwrap();
            let retrieved = store.get_node(1).unwrap();
            assert_eq!(retrieved, Some(metadata));
        }
    }
}
