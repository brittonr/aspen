//! Transaction builder for optimistic concurrency control (OCC).
//!
//! Provides a fluent API for building FoundationDB-style optimistic transactions.
//!
//! ## Usage
//!
//! ```ignore
//! use aspen::client::{TransactionBuilder, TransactionResult};
//! use aspen::api::KeyValueStore;
//!
//! // Build a transaction that reads current versions then conditionally writes
//! let result = TransactionBuilder::new()
//!     .read("user:123", current_version)
//!     .read("balance:123", balance_version)
//!     .set("user:123", "updated-data")
//!     .set("balance:123", new_balance.to_string())
//!     .execute(&kv_store)
//!     .await?;
//!
//! match result {
//!     TransactionResult::Committed => println!("Transaction succeeded"),
//!     TransactionResult::Conflict { key, expected, actual } => {
//!         println!("Version conflict on {key}: expected {expected}, got {actual}");
//!     }
//! }
//! ```
//!
//! ## OCC Pattern
//!
//! 1. **Read phase**: Client reads keys and captures their versions
//! 2. **Write phase**: Client builds transaction with expected versions
//! 3. **Validation phase**: Server validates all versions still match
//! 4. **Apply phase**: If valid, server atomically applies all writes

use crate::api::KeyValueStore;
use crate::api::KeyValueStoreError;
use crate::api::WriteCommand;
use crate::api::WriteOp;
use crate::api::WriteRequest;

/// Result of a transaction execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionResult {
    /// Transaction was successfully committed.
    Committed,
    /// Transaction failed due to version conflict.
    Conflict {
        /// Key that had a version mismatch.
        key: String,
        /// Expected version from read phase.
        expected: i64,
        /// Actual version found during validation.
        actual: i64,
    },
}

/// Builder for optimistic transactions.
///
/// Provides a fluent API for constructing transactions with a read set
/// (versions to validate) and a write set (operations to apply).
#[derive(Debug, Clone, Default)]
pub struct TransactionBuilder {
    /// Keys and their expected versions to validate before applying writes.
    read_set: Vec<(String, i64)>,
    /// Operations to apply if all versions match.
    write_set: Vec<WriteOp>,
}

impl TransactionBuilder {
    /// Create a new empty transaction builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a key to the read set with its expected version.
    ///
    /// The transaction will only succeed if the key's current version
    /// matches the expected version when the transaction is validated.
    ///
    /// # Arguments
    /// * `key` - The key to validate
    /// * `expected_version` - The version the key must have (0 if key should not exist)
    pub fn read(mut self, key: impl Into<String>, expected_version: i64) -> Self {
        self.read_set.push((key.into(), expected_version));
        self
    }

    /// Add multiple keys to the read set.
    ///
    /// # Arguments
    /// * `reads` - Iterator of (key, expected_version) pairs
    pub fn read_many<I, S>(mut self, reads: I) -> Self
    where
        I: IntoIterator<Item = (S, i64)>,
        S: Into<String>,
    {
        for (key, version) in reads {
            self.read_set.push((key.into(), version));
        }
        self
    }

    /// Add a set operation to the write set.
    ///
    /// # Arguments
    /// * `key` - The key to set
    /// * `value` - The value to set
    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.write_set.push(WriteOp::Set {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    /// Add a delete operation to the write set.
    ///
    /// # Arguments
    /// * `key` - The key to delete
    pub fn delete(mut self, key: impl Into<String>) -> Self {
        self.write_set.push(WriteOp::Delete { key: key.into() });
        self
    }

    /// Add multiple set operations to the write set.
    ///
    /// # Arguments
    /// * `pairs` - Iterator of (key, value) pairs to set
    pub fn set_many<I, K, V>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (key, value) in pairs {
            self.write_set.push(WriteOp::Set {
                key: key.into(),
                value: value.into(),
            });
        }
        self
    }

    /// Add multiple delete operations to the write set.
    ///
    /// # Arguments
    /// * `keys` - Iterator of keys to delete
    pub fn delete_many<I, K>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = K>,
        K: Into<String>,
    {
        for key in keys {
            self.write_set.push(WriteOp::Delete { key: key.into() });
        }
        self
    }

    /// Check if the transaction has any operations.
    pub fn is_empty(&self) -> bool {
        self.write_set.is_empty() && self.read_set.is_empty()
    }

    /// Get the number of keys in the read set.
    pub fn read_count(&self) -> usize {
        self.read_set.len()
    }

    /// Get the number of operations in the write set.
    pub fn write_count(&self) -> usize {
        self.write_set.len()
    }

    /// Execute the transaction against a KeyValueStore.
    ///
    /// # Arguments
    /// * `store` - The KeyValueStore to execute against
    ///
    /// # Returns
    /// * `Ok(TransactionResult::Committed)` - Transaction succeeded
    /// * `Ok(TransactionResult::Conflict { .. })` - Version conflict detected
    /// * `Err(_)` - Other error (network, storage, etc.)
    pub async fn execute<KV: KeyValueStore>(self, store: &KV) -> Result<TransactionResult, KeyValueStoreError> {
        let request = WriteRequest {
            command: WriteCommand::OptimisticTransaction {
                read_set: self.read_set,
                write_set: self.write_set,
            },
        };

        let result = store.write(request).await?;

        // Check for OCC conflict
        if result.occ_conflict == Some(true) {
            return Ok(TransactionResult::Conflict {
                key: result.conflict_key.unwrap_or_default(),
                expected: result.conflict_expected_version.unwrap_or(0),
                actual: result.conflict_actual_version.unwrap_or(0),
            });
        }

        Ok(TransactionResult::Committed)
    }

    /// Build the WriteCommand without executing.
    ///
    /// Useful for batching or custom execution patterns.
    pub fn build(self) -> WriteCommand {
        WriteCommand::OptimisticTransaction {
            read_set: self.read_set,
            write_set: self.write_set,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_fluent_api() {
        let txn = TransactionBuilder::new().read("key1", 1).read("key2", 2).set("key1", "value1").delete("key3");

        assert_eq!(txn.read_count(), 2);
        assert_eq!(txn.write_count(), 2);
    }

    #[test]
    fn test_builder_many_operations() {
        let reads = vec![("k1", 1), ("k2", 2), ("k3", 3)];
        let writes = vec![("k1", "v1"), ("k2", "v2")];
        let deletes = vec!["k3", "k4"];

        let txn = TransactionBuilder::new().read_many(reads).set_many(writes).delete_many(deletes);

        assert_eq!(txn.read_count(), 3);
        assert_eq!(txn.write_count(), 4);
    }

    #[test]
    fn test_build_command() {
        let txn = TransactionBuilder::new().read("key1", 5).set("key1", "new_value");

        let cmd = txn.build();
        match cmd {
            WriteCommand::OptimisticTransaction { read_set, write_set } => {
                assert_eq!(read_set.len(), 1);
                assert_eq!(read_set[0], ("key1".to_string(), 5));
                assert_eq!(write_set.len(), 1);
                match &write_set[0] {
                    WriteOp::Set { key, value } => {
                        assert_eq!(key, "key1");
                        assert_eq!(value, "new_value");
                    }
                    _ => panic!("Expected Set operation"),
                }
            }
            _ => panic!("Expected OptimisticTransaction"),
        }
    }

    #[test]
    fn test_empty_transaction() {
        let txn = TransactionBuilder::new();
        assert!(txn.is_empty());

        let txn_with_read = TransactionBuilder::new().read("key", 1);
        assert!(!txn_with_read.is_empty());

        let txn_with_write = TransactionBuilder::new().set("key", "value");
        assert!(!txn_with_write.is_empty());
    }
}
