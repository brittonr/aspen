//! Internal/private methods for DirectoryLayer.

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use snafu::ResultExt;

use super::super::Subspace;
use super::super::Tuple;
use super::AllocationFailedSnafu;
use super::DirectoryError;
use super::StorageSnafu;
use super::layer::DirectoryLayer;
use super::subspace::DirectorySubspace;
use crate::constants::directory::DIR_FIELD_CREATED_AT_MS;
use crate::constants::directory::DIR_FIELD_LAYER;
use crate::constants::directory::DIR_FIELD_PREFIX;
use crate::error::KeyValueStoreError;
use crate::kv::BatchCondition;
use crate::kv::BatchOperation;
use crate::kv::ReadRequest;
use crate::kv::WriteCommand;
use crate::kv::WriteRequest;
use crate::traits::KeyValueStore;

impl<KV: KeyValueStore + ?Sized> DirectoryLayer<KV> {
    /// Open an existing directory (internal implementation).
    pub(super) async fn open_internal(&self, path: &[&str]) -> Result<DirectorySubspace, DirectoryError> {
        let layer_key = self.metadata_key(path, DIR_FIELD_LAYER);
        let prefix_key = self.metadata_key(path, DIR_FIELD_PREFIX);
        let created_key = self.metadata_key(path, DIR_FIELD_CREATED_AT_MS);

        // Read layer type
        let layer = match self.store.read(ReadRequest::new(layer_key)).await {
            Ok(result) => match result.kv {
                Some(kv) => kv.value,
                None => {
                    return Err(DirectoryError::NotFound {
                        path: path.iter().map(|s| s.to_string()).collect(),
                    });
                }
            },
            Err(KeyValueStoreError::NotFound { .. }) => {
                return Err(DirectoryError::NotFound {
                    path: path.iter().map(|s| s.to_string()).collect(),
                });
            }
            Err(e) => return Err(DirectoryError::Storage { source: e }),
        };

        // Read prefix
        let prefix_hex = match self.store.read(ReadRequest::new(prefix_key)).await {
            Ok(result) => result.kv.map(|kv| kv.value).unwrap_or_default(),
            Err(e) => return Err(DirectoryError::Storage { source: e }),
        };

        let prefix = hex::decode(&prefix_hex).map_err(|_| DirectoryError::CorruptedMetadata {
            path: path.iter().map(|s| s.to_string()).collect(),
            reason: format!("invalid prefix hex: {prefix_hex}"),
        })?;

        // Read creation time
        let created_at_ms = match self.store.read(ReadRequest::new(created_key)).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                value_str.parse::<u64>().unwrap_or(0)
            }
            Err(KeyValueStoreError::NotFound { .. }) => 0,
            Err(e) => return Err(DirectoryError::Storage { source: e }),
        };

        Ok(DirectorySubspace::new(
            Subspace::from_bytes(prefix),
            path.iter().map(|s| s.to_string()).collect(),
            layer,
            created_at_ms,
        ))
    }

    /// Create a new directory (internal implementation).
    pub(super) async fn create_internal(
        &self,
        path: &[&str],
        layer: &str,
    ) -> Result<DirectorySubspace, DirectoryError> {
        // Allocate a unique prefix
        let prefix_int = self.allocator.allocate().await.context(AllocationFailedSnafu)?;
        let prefix = self.allocator.encode_prefix(prefix_int);

        let created_at_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        // Build metadata keys
        let layer_key = self.metadata_key(path, DIR_FIELD_LAYER);
        let prefix_key = self.metadata_key(path, DIR_FIELD_PREFIX);
        let created_key = self.metadata_key(path, DIR_FIELD_CREATED_AT_MS);

        // Atomic create: conditional batch
        self.store
            .write(WriteRequest {
                command: WriteCommand::ConditionalBatch {
                    conditions: vec![BatchCondition::KeyNotExists { key: layer_key.clone() }],
                    operations: vec![
                        BatchOperation::Set {
                            key: layer_key,
                            value: layer.to_string(),
                        },
                        BatchOperation::Set {
                            key: prefix_key,
                            value: hex::encode(&prefix),
                        },
                        BatchOperation::Set {
                            key: created_key,
                            value: created_at_ms.to_string(),
                        },
                    ],
                },
            })
            .await
            .context(StorageSnafu)?;

        Ok(DirectorySubspace::new(
            Subspace::from_bytes(prefix),
            path.iter().map(|s| s.to_string()).collect(),
            layer.to_string(),
            created_at_ms,
        ))
    }

    /// Convert a path to a tuple for key construction.
    pub(super) fn path_to_tuple(&self, path: &[&str]) -> Tuple {
        let mut tuple = Tuple::new();
        for component in path {
            tuple.push_mut(*component);
        }
        tuple
    }

    /// Generate a metadata key for a path and field.
    pub(super) fn metadata_key(&self, path: &[&str], field: &str) -> String {
        let mut tuple = self.path_to_tuple(path);
        tuple.push_mut(field);
        String::from_utf8_lossy(&self.node_subspace.pack(&tuple)).to_string()
    }
}
