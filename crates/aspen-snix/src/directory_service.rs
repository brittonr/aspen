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
use std::time::Duration;
use std::time::Instant;

use aspen_core::circuit_breaker::CircuitBreaker;
use async_trait::async_trait;
use base64::Engine;
use prost::Message;
use snix_castore::B3Digest;
use snix_castore::Directory;
use snix_castore::Node;
use snix_castore::directoryservice::DirectoryPutter;
use snix_castore::directoryservice::DirectoryService;
use snix_castore::directoryservice::Error;
use tokio::sync::Mutex;
use tracing::debug;
use tracing::instrument;
use tracing::warn;

use crate::BoxStream;
use crate::constants::DIRECTORY_KEY_PREFIX;
use crate::constants::MAX_DIRECTORY_DEPTH;
use crate::constants::MAX_RECURSIVE_BUFFER;

/// Default consecutive failure threshold for the directory service circuit breaker.
const DIR_CB_THRESHOLD: u32 = 5;

/// Default open duration for the directory service circuit breaker.
const DIR_CB_OPEN_DURATION: Duration = Duration::from_secs(30);

/// SNIX DirectoryService implementation backed by Aspen's Raft KV store.
///
/// Stores directory metadata as protobuf-encoded values, keyed by BLAKE3 digest.
/// Provides linearizable access to directory structures across the cluster.
pub struct RaftDirectoryService<K: ?Sized> {
    kv: Arc<K>,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

// Manual Clone impl: Arc is always cloneable regardless of K
impl<K: ?Sized> Clone for RaftDirectoryService<K> {
    fn clone(&self) -> Self {
        Self {
            kv: Arc::clone(&self.kv),
            circuit_breaker: Arc::clone(&self.circuit_breaker),
        }
    }
}

impl<K> RaftDirectoryService<K> {
    /// Create a new RaftDirectoryService with the given KV store.
    pub fn new(kv: K) -> Self {
        Self {
            kv: Arc::new(kv),
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(DIR_CB_THRESHOLD, DIR_CB_OPEN_DURATION))),
        }
    }
}

impl<K: ?Sized> RaftDirectoryService<K> {
    /// Create a new RaftDirectoryService from an Arc'd KV store.
    pub fn from_arc(kv: Arc<K>) -> Self {
        Self {
            kv,
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(DIR_CB_THRESHOLD, DIR_CB_OPEN_DURATION))),
        }
    }

    /// Check if the circuit breaker is open and return an error if so.
    async fn check_circuit(&self) -> Result<(), Error> {
        let cb = self.circuit_breaker.lock().await;
        if cb.should_reject(Instant::now()) {
            return Err("directory service circuit breaker is open — too many consecutive failures".into());
        }
        Ok(())
    }

    /// Record a successful operation. Logs recovery if breaker was open.
    async fn record_success(&self) {
        if self.circuit_breaker.lock().await.record_success() {
            tracing::info!("directory service circuit breaker recovered — closed");
        }
    }

    /// Record a failed operation. Logs a warning when the breaker trips open.
    async fn record_failure(&self) {
        let mut cb = self.circuit_breaker.lock().await;
        let was_open = cb.should_reject(Instant::now());
        cb.record_failure(Instant::now());
        if !was_open && cb.should_reject(Instant::now()) {
            warn!(
                failures = cb.consecutive_failures(),
                open_duration_secs = cb.open_duration().as_secs(),
                "directory service circuit breaker tripped open"
            );
        }
    }

    /// Build the KV key for a directory digest.
    fn make_key(digest: &B3Digest) -> String {
        format!("{}{}", DIRECTORY_KEY_PREFIX, hex::encode(digest.as_ref()))
    }
}

#[async_trait]
impl<K> DirectoryService for RaftDirectoryService<K>
where K: aspen_core::KeyValueStore + Send + Sync + 'static + ?Sized
{
    #[instrument(skip(self), fields(digest = %digest))]
    async fn get(&self, digest: &B3Digest) -> Result<Option<Directory>, Error> {
        self.check_circuit().await?;
        let key = Self::make_key(digest);

        let result = match self.kv.read(aspen_core::kv::ReadRequest::new(&key)).await {
            Ok(result) => {
                self.record_success().await;
                result
            }
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => {
                self.record_success().await;
                debug!("directory not found");
                return Ok(None);
            }
            Err(e) => {
                self.record_failure().await;
                return Err(format!("KV read error: {e}").into());
            }
        };

        match result.kv {
            Some(kv) => {
                // Decode from base64 (KV stores strings, not bytes)
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&kv.value)
                    .map_err(|e| -> Error { format!("base64 decode error: {e}").into() })?;

                // Decode protobuf
                let proto_dir = snix_castore::proto::Directory::decode(bytes.as_slice())
                    .map_err(|e| -> Error { format!("protobuf decode error: {e}").into() })?;

                // Convert to Directory
                let dir = Directory::try_from(proto_dir)
                    .map_err(|e| -> Error { format!("directory conversion error: {e}").into() })?;

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
        self.check_circuit().await?;

        // Convert to protobuf and compute digest
        let proto_dir = snix_castore::proto::Directory::from(directory);
        let digest = proto_dir.digest();

        // Encode to bytes
        let bytes = proto_dir.encode_to_vec();

        // Encode to base64 for KV storage
        let value = base64::engine::general_purpose::STANDARD.encode(&bytes);

        // Write to KV store
        let key = Self::make_key(&digest);
        match self
            .kv
            .write(aspen_core::kv::WriteRequest {
                command: aspen_core::kv::WriteCommand::Set { key, value },
            })
            .await
        {
            Ok(_) => {
                self.record_success().await;
            }
            Err(e) => {
                self.record_failure().await;
                return Err(format!("KV write error: {e}").into());
            }
        }

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
                    let err: Error = format!(
                        "maximum directory depth {} exceeded",
                        MAX_DIRECTORY_DEPTH
                    ).into();
                    Err(err)?;
                }

                // Check buffer limit
                if queue.len() as u32 > MAX_RECURSIVE_BUFFER {
                    let err: Error = format!(
                        "maximum recursive buffer {} exceeded",
                        MAX_RECURSIVE_BUFFER
                    ).into();
                    Err(err)?;
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
pub struct RaftDirectoryPutter<K: ?Sized> {
    kv: Arc<K>,
    last_digest: Option<B3Digest>,
}

impl<K: ?Sized> RaftDirectoryPutter<K> {
    fn new(kv: Arc<K>) -> Self {
        Self { kv, last_digest: None }
    }
}

#[async_trait]
impl<K> DirectoryPutter for RaftDirectoryPutter<K>
where K: aspen_core::KeyValueStore + Send + Sync + 'static + ?Sized
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
        tracing::info!(key = %key, digest = %digest, "directory putter: storing directory");
        self.kv
            .write(aspen_core::kv::WriteRequest {
                command: aspen_core::kv::WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| -> Error { format!("KV write error: {e}").into() })?;

        // Track as potential root
        self.last_digest = Some(digest);

        Ok(())
    }

    async fn close(&mut self) -> Result<B3Digest, Error> {
        let digest = self.last_digest.take().ok_or_else(|| -> Error { "no directories were put".into() })?;
        tracing::info!(root_digest = %digest, "directory putter: closed with root");
        Ok(digest)
    }
}

#[cfg(test)]
mod tests {
    // Tests require a KV store implementation, which is provided by aspen-testing
    // Integration tests are in the tests/ directory
}
