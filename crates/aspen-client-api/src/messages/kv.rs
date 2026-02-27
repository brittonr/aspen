//! KV operation types.
//!
//! Request/response types for key-value store operations including read, write,
//! delete, scan, compare-and-swap, and vault operations.

use serde::Deserialize;
use serde::Serialize;

/// KV domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvRequest {
    /// Read a key from the key-value store.
    ReadKey {
        /// Key to read.
        key: String,
    },

    /// Write a key-value pair to the store.
    WriteKey {
        /// Key to write.
        key: String,
        /// Value to write.
        value: Vec<u8>,
    },

    /// Compare-and-swap: atomically update value if current value matches expected.
    ///
    /// - `expected: None` means the key must NOT exist (create-if-absent)
    /// - `expected: Some(val)` means the key must exist with exactly that value
    CompareAndSwapKey {
        /// Key to update.
        key: String,
        /// Expected current value (None = must not exist).
        expected: Option<Vec<u8>>,
        /// New value to set if condition matches.
        new_value: Vec<u8>,
    },

    /// Compare-and-delete: atomically delete key if current value matches expected.
    CompareAndDeleteKey {
        /// Key to delete.
        key: String,
        /// Expected current value.
        expected: Vec<u8>,
    },

    /// Delete a key from the key-value store.
    DeleteKey {
        /// Key to delete.
        key: String,
    },

    /// Scan keys with prefix and pagination.
    ScanKeys {
        /// Key prefix to match (empty string matches all).
        prefix: String,
        /// Maximum results (default 1000, max 10000).
        limit: Option<u32>,
        /// Continuation token from previous scan.
        continuation_token: Option<String>,
    },

    /// List all vaults (key namespaces).
    ListVaults,

    /// Get keys in a specific vault.
    GetVaultKeys {
        /// Name of the vault to query.
        vault_name: String,
    },

    /// Write a key attached to a lease.
    ///
    /// Key will be deleted when the lease expires or is revoked.
    WriteKeyWithLease {
        /// Key to write.
        key: String,
        /// Value to write.
        value: Vec<u8>,
        /// Lease ID to attach the key to.
        lease_id: u64,
    },
}

#[cfg(feature = "auth")]
impl KvRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::ReadKey { key } | Self::ScanKeys { prefix: key, .. } | Self::GetVaultKeys { vault_name: key } => {
                Some(Operation::Read { key: key.clone() })
            }

            Self::WriteKey { key, value } | Self::WriteKeyWithLease { key, value, .. } => Some(Operation::Write {
                key: key.clone(),
                value: value.clone(),
            }),
            Self::DeleteKey { key } | Self::CompareAndSwapKey { key, .. } | Self::CompareAndDeleteKey { key, .. } => {
                Some(Operation::Write {
                    key: key.clone(),
                    value: vec![],
                })
            }
            Self::ListVaults => Some(Operation::Read { key: String::new() }),
        }
    }
}

/// Delete key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResultResponse {
    /// The key that was targeted.
    pub key: String,
    /// True if key existed and was deleted, false if not found.
    pub was_deleted: bool,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Scan keys result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultResponse {
    /// Matching key-value pairs.
    pub entries: Vec<ScanEntry>,
    /// Number of entries returned.
    pub count: u32,
    /// True if more results available.
    pub is_truncated: bool,
    /// Token for next page (if truncated).
    pub continuation_token: Option<String>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

// ============================================================================
// Index Operation Types
// ============================================================================

/// Result of an index creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCreateResultResponse {
    /// Whether the creation succeeded.
    pub is_success: bool,
    /// Name of the created index.
    pub name: String,
    /// Error message if creation failed.
    pub error: Option<String>,
}

/// Result of an index drop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDropResultResponse {
    /// Whether the drop succeeded.
    pub is_success: bool,
    /// Name of the dropped index.
    pub name: String,
    /// Whether the index was actually removed (false if not found).
    pub was_dropped: bool,
    /// Error message if drop failed.
    pub error: Option<String>,
}

/// Result of an index scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexScanResultResponse {
    /// Whether the scan succeeded.
    pub is_success: bool,
    /// Primary keys matching the query.
    pub primary_keys: Vec<String>,
    /// Whether more results are available beyond the limit.
    pub has_more: bool,
    /// Number of results returned.
    pub count: u32,
    /// Error message if scan failed.
    pub error: Option<String>,
}

/// Result of listing indexes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexListResultResponse {
    /// Whether the list succeeded.
    pub is_success: bool,
    /// Index definitions.
    pub indexes: Vec<IndexDefinitionWire>,
    /// Number of indexes returned.
    pub count: u32,
    /// Error message if list failed.
    pub error: Option<String>,
}

/// Wire-format index definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinitionWire {
    /// Index name.
    pub name: String,
    /// Field being indexed (e.g., "mod_revision").
    pub field: Option<String>,
    /// Field type: "integer", "unsignedinteger", or "string".
    pub field_type: String,
    /// Whether this is a built-in index.
    pub builtin: bool,
    /// Whether the index enforces uniqueness.
    pub is_unique: bool,
    /// Whether null values are indexed.
    pub should_index_nulls: bool,
}

// ============================================================================
// Scan Types
// ============================================================================

/// Single entry from scan operation with revision metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEntry {
    /// Key name.
    pub key: String,
    /// Value (as UTF-8 string).
    pub value: String,
    /// Per-key version counter (1, 2, 3...). Reset to 1 on delete+recreate.
    #[serde(default)]
    pub version: u64,
    /// Raft log index when key was first created.
    #[serde(default)]
    pub create_revision: u64,
    /// Raft log index of last modification.
    #[serde(default)]
    pub mod_revision: u64,
}
