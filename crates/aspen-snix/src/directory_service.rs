//! SNIX DirectoryService implementation backed by Aspen's Raft KV store.
//!
//! This module provides [`RaftDirectoryService`], which implements the SNIX
//! [`DirectoryService`] trait using Aspen's distributed key-value store for
//! directory metadata storage.
//!
//! # Architecture
//!
//! Directories are stored as protobuf-encoded values in the Raft KV store,
//! keyed by their BLAKE3 digest. This provides:
//!
//! - Linearizable reads and writes
//! - Automatic replication across cluster nodes
//! - Consistent ordering for recursive traversals
//!
//! # Key Format
//!
//! Directory keys use the format: `snix:dir:<hex-encoded-digest>`

use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use futures::stream::BoxStream;
use prost::Message;
use snix_castore::directoryservice::{DirectoryPutter, DirectoryService};
use snix_castore::{B3Digest, Directory, Error, Node};
use tracing::{debug, instrument};

use crate::constants::{DIRECTORY_KEY_PREFIX, MAX_DIRECTORY_DEPTH, MAX_RECURSIVE_BUFFER};

/// SNIX DirectoryService implementation backed by Aspen's Raft KV store.
///
/// Stores directory metadata as protobuf-encoded values, keyed by BLAKE3 digest.
/// Provides linearizable access to directory structures across the cluster.
pub struct RaftDirectoryService<K> {
    kv: Arc<K>,
}

// Manual Clone impl: Arc is always cloneable regardless of K
impl<K> Clone for RaftDirectoryService<K> {
    fn clone(&self) -> Self {
        Self {
            kv: Arc::clone(&self.kv),
        }
    }
}

impl<K> RaftDirectoryService<K> {
    /// Create a new RaftDirectoryService with the given KV store.
    pub fn new(kv: K) -> Self {
        Self { kv: Arc::new(kv) }
    }

    /// Create a new RaftDirectoryService from an Arc'd KV store.
    pub fn from_arc(kv: Arc<K>) -> Self {
        Self { kv }
    }

    /// Build the KV key for a directory digest.
    fn make_key(digest: &B3Digest) -> String {
        format!("{}{}", DIRECTORY_KEY_PREFIX, hex::encode(digest.as_ref()))
    }
}

#[async_trait]
impl<K> DirectoryService for RaftDirectoryService<K>
where
    K: aspen_core::KeyValueStore + Send + Sync + 'static,
{
    #[instrument(skip(self), fields(digest = %digest))]
    async fn get(&self, digest: &B3Digest) -> Result<Option<Directory>, Error> {
        let key = Self::make_key(digest);

        let result = self
            .kv
            .read(aspen_core::kv::ReadRequest::new(&key))
            .await
            .map_err(|e| Error::StorageError(format!("KV read error: {}", e)))?;

        match result.kv {
            Some(kv) => {
                // Decode from base64 (KV stores strings, not bytes)
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&kv.value)
                    .map_err(|e| Error::StorageError(format!("base64 decode error: {}", e)))?;

                // Decode protobuf
                let proto_dir = snix_castore::proto::Directory::decode(bytes.as_slice())
                    .map_err(|e| Error::StorageError(format!("protobuf decode error: {}", e)))?;

                // Convert to Directory
                let dir = Directory::try_from(proto_dir)
                    .map_err(|e| Error::StorageError(format!("directory conversion error: {}", e)))?;

                debug!(size = dir.size(), "directory retrieved");
                Ok(Some(dir))
            }
            None => {
                debug!("directory not found");
                Ok(None)
            }
        }
    }

    #[instrument(skip(self, directory), fields(size = directory.size()))]
    async fn put(&self, directory: Directory) -> Result<B3Digest, Error> {
        // Convert to protobuf and compute digest
        let proto_dir = snix_castore::proto::Directory::from(directory);
        let digest = proto_dir.digest();

        // Encode to bytes
        let bytes = proto_dir.encode_to_vec();

        // Encode to base64 for KV storage
        let value = base64::engine::general_purpose::STANDARD.encode(&bytes);

        // Write to KV store
        let key = Self::make_key(&digest);
        self.kv
            .write(aspen_core::kv::WriteRequest {
                command: aspen_core::kv::WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| Error::StorageError(format!("KV write error: {}", e)))?;

        debug!(digest = %digest, "directory stored");
        Ok(digest)
    }

    /// Returns directories in root-to-leaves order (BFS traversal).
    ///
    /// This ordering allows receivers to validate that each directory
    /// is connected to the initially requested root.
    fn get_recursive(&self, root_directory_digest: &B3Digest) -> BoxStream<'_, Result<Directory, Error>> {
        let root_digest = *root_directory_digest;

        Box::pin(async_stream::try_stream! {
            let mut queue: VecDeque<B3Digest> = VecDeque::new();
            let mut depth: u32 = 0;
            let mut visited: std::collections::HashSet<B3Digest> = std::collections::HashSet::new();

            queue.push_back(root_digest);

            while let Some(digest) = queue.pop_front() {
                // Check for cycles
                if visited.contains(&digest) {
                    continue;
                }
                visited.insert(digest);

                // Check depth limit
                if depth > MAX_DIRECTORY_DEPTH {
                    Err(Error::StorageError(format!(
                        "maximum directory depth {} exceeded",
                        MAX_DIRECTORY_DEPTH
                    )))?;
                }

                // Check buffer limit
                if queue.len() as u32 > MAX_RECURSIVE_BUFFER {
                    Err(Error::StorageError(format!(
                        "maximum recursive buffer {} exceeded",
                        MAX_RECURSIVE_BUFFER
                    )))?;
                }

                if let Some(dir) = self.get(&digest).await? {
                    // Queue child directories for BFS traversal
                    // This ensures root-to-leaves ordering
                    for (_name, node) in dir.nodes() {
                        if let Node::Directory { digest: child_digest, .. } = node
                            && !visited.contains(child_digest)
                        {
                            queue.push_back(*child_digest);
                        }
                    }

                    yield dir;
                    depth += 1;
                }
            }
        })
    }

    fn put_multiple_start(&self) -> Box<dyn DirectoryPutter + '_> {
        Box::new(RaftDirectoryPutter::new(Arc::clone(&self.kv)))
    }
}

/// DirectoryPutter implementation that batches directory writes.
///
/// Directories are written in leaves-to-root order by the caller.
/// We store each directory as it comes in and track the last digest
/// as the root.
pub struct RaftDirectoryPutter<K> {
    kv: Arc<K>,
    last_digest: Option<B3Digest>,
}

impl<K> RaftDirectoryPutter<K> {
    fn new(kv: Arc<K>) -> Self {
        Self { kv, last_digest: None }
    }
}

#[async_trait]
impl<K> DirectoryPutter for RaftDirectoryPutter<K>
where
    K: aspen_core::KeyValueStore + Send + Sync + 'static,
{
    async fn put(&mut self, directory: Directory) -> Result<(), Error> {
        // Convert to protobuf and compute digest
        let proto_dir = snix_castore::proto::Directory::from(directory);
        let digest = proto_dir.digest();

        // Encode to bytes
        let bytes = proto_dir.encode_to_vec();

        // Encode to base64 for KV storage
        let value = base64::engine::general_purpose::STANDARD.encode(&bytes);

        // Write to KV store
        let key = format!("{}{}", DIRECTORY_KEY_PREFIX, hex::encode(digest.as_ref()));
        self.kv
            .write(aspen_core::kv::WriteRequest {
                command: aspen_core::kv::WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| Error::StorageError(format!("KV write error: {}", e)))?;

        // Track as potential root
        self.last_digest = Some(digest);

        Ok(())
    }

    async fn close(&mut self) -> Result<B3Digest, Error> {
        self.last_digest.take().ok_or_else(|| Error::StorageError("no directories were put".to_string()))
    }
}

#[cfg(test)]
mod tests {
    // Tests require a KV store implementation, which is provided by aspen-testing
    // Integration tests are in the tests/ directory
}
