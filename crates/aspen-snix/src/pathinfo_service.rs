//! SNIX PathInfoService implementation backed by Aspen's Raft KV store.
//!
//! This module provides [`RaftPathInfoService`], which implements the SNIX
//! [`PathInfoService`] trait using Aspen's distributed key-value store for
//! path info metadata storage.
//!
//! # Architecture
//!
//! PathInfo entries are stored as protobuf-encoded values in the Raft KV store,
//! keyed by the 20-byte Nix store path digest. This provides:
//!
//! - Linearizable reads and writes
//! - Automatic replication across cluster nodes
//! - Consistent path info lookups
//!
//! # Key Format
//!
//! PathInfo keys use the format: `snix:pathinfo:<hex-encoded-20-byte-digest>`
//!
//! Note: The digest is 20 bytes (160 bits), the truncated SHA-256 used by Nix
//! for store paths, NOT the 32-byte BLAKE3 digest used for content addressing.

use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use futures::StreamExt;
use futures::stream::BoxStream;
use prost::Message;
use snix_store::pathinfoservice::Error;
use snix_store::pathinfoservice::PathInfo;
use snix_store::pathinfoservice::PathInfoService;
use tracing::debug;
use tracing::instrument;

use crate::constants::PATHINFO_KEY_PREFIX;
use crate::constants::STORE_PATH_DIGEST_LENGTH;

/// SNIX PathInfoService implementation backed by Aspen's Raft KV store.
///
/// Stores PathInfo metadata as protobuf-encoded values, keyed by the 20-byte
/// Nix store path digest. Provides linearizable access to store path metadata
/// across the cluster.
#[derive(Clone)]
pub struct RaftPathInfoService<K> {
    kv: Arc<K>,
}

impl<K> RaftPathInfoService<K> {
    /// Create a new RaftPathInfoService with the given KV store.
    pub fn new(kv: K) -> Self {
        Self { kv: Arc::new(kv) }
    }

    /// Create a new RaftPathInfoService from an Arc'd KV store.
    pub fn from_arc(kv: Arc<K>) -> Self {
        Self { kv }
    }

    /// Build the KV key for a store path digest.
    ///
    /// The digest is 20 bytes (the Nix store path digest).
    fn make_key(digest: &[u8; STORE_PATH_DIGEST_LENGTH]) -> String {
        format!("{}{}", PATHINFO_KEY_PREFIX, hex::encode(digest))
    }
}

#[async_trait]
impl<K> PathInfoService for RaftPathInfoService<K>
where K: aspen_core::KeyValueStore + Send + Sync + 'static
{
    #[instrument(skip(self), fields(digest = hex::encode(digest)))]
    async fn get(&self, digest: [u8; 20]) -> Result<Option<PathInfo>, Error> {
        let key = Self::make_key(&digest);

        let result = match self.kv.read(aspen_core::kv::ReadRequest::new(&key)).await {
            Ok(result) => result,
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => {
                // Key not found is not an error for SNIX - return None
                debug!("path info not found");
                return Ok(None);
            }
            Err(e) => {
                return Err(Box::new(std::io::Error::other(format!("KV read error: {}", e))));
            }
        };

        match result.kv {
            Some(kv) => {
                // Decode from base64 (KV stores strings, not bytes)
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&kv.value)
                    .map_err(|e| -> Error { Box::new(std::io::Error::other(format!("base64 decode error: {}", e))) })?;

                // Decode protobuf
                let proto_pathinfo = snix_store::proto::PathInfo::decode(bytes.as_slice()).map_err(|e| -> Error {
                    Box::new(std::io::Error::other(format!("protobuf decode error: {}", e)))
                })?;

                // Convert to PathInfo
                let path_info = PathInfo::try_from(proto_pathinfo).map_err(|e| -> Error {
                    Box::new(std::io::Error::other(format!("pathinfo conversion error: {}", e)))
                })?;

                debug!(store_path = %path_info.store_path, "path info retrieved");
                Ok(Some(path_info))
            }
            None => {
                debug!("path info not found");
                Ok(None)
            }
        }
    }

    #[instrument(skip(self, path_info), fields(store_path = %path_info.store_path))]
    async fn put(&self, path_info: PathInfo) -> Result<PathInfo, Error> {
        // Get the store path digest (20 bytes)
        let digest: [u8; 20] = *path_info.store_path.digest();

        // Convert to protobuf
        let proto_pathinfo = snix_store::proto::PathInfo::from(path_info.clone());

        // Encode to bytes
        let bytes = proto_pathinfo.encode_to_vec();

        // Encode to base64 for KV storage
        let value = base64::engine::general_purpose::STANDARD.encode(&bytes);

        // Write to KV store
        let key = Self::make_key(&digest);
        self.kv
            .write(aspen_core::kv::WriteRequest {
                command: aspen_core::kv::WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| -> Error { Box::new(std::io::Error::other(format!("KV write error: {}", e))) })?;

        debug!(store_path = %path_info.store_path, "path info stored");
        Ok(path_info)
    }

    /// List all PathInfo entries in the store.
    ///
    /// This scans all keys with the pathinfo prefix and returns them as a stream.
    fn list(&self) -> BoxStream<'static, Result<PathInfo, Error>> {
        let kv = Arc::clone(&self.kv);

        // Use a channel-based approach for type inference
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<PathInfo, Error>>(64);

        tokio::spawn(async move {
            let prefix = PATHINFO_KEY_PREFIX.to_string();
            let mut continuation_token: Option<String> = None;
            let page_size: u32 = 100;

            loop {
                let result = match kv
                    .scan(aspen_core::kv::ScanRequest {
                        prefix: prefix.clone(),
                        limit_results: Some(page_size),
                        continuation_token: continuation_token.clone(),
                    })
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx
                            .send(Err(Box::new(std::io::Error::other(format!("KV scan error: {}", e))) as Error))
                            .await;
                        return;
                    }
                };

                for entry in &result.entries {
                    // Decode from base64
                    let bytes = match base64::engine::general_purpose::STANDARD.decode(&entry.value) {
                        Ok(b) => b,
                        Err(e) => {
                            let _ = tx
                                .send(Err(
                                    Box::new(std::io::Error::other(format!("base64 decode error: {}", e))) as Error
                                ))
                                .await;
                            return;
                        }
                    };

                    // Decode protobuf
                    let proto_pathinfo = match snix_store::proto::PathInfo::decode(bytes.as_slice()) {
                        Ok(p) => p,
                        Err(e) => {
                            let _ = tx
                                .send(Err(
                                    Box::new(std::io::Error::other(format!("protobuf decode error: {}", e))) as Error
                                ))
                                .await;
                            return;
                        }
                    };

                    // Convert to PathInfo
                    let path_info = match PathInfo::try_from(proto_pathinfo) {
                        Ok(p) => p,
                        Err(e) => {
                            let _ = tx
                                .send(Err(Box::new(std::io::Error::other(format!("pathinfo conversion error: {}", e)))
                                    as Error))
                                .await;
                            return;
                        }
                    };

                    if tx.send(Ok(path_info)).await.is_err() {
                        // Receiver dropped
                        return;
                    }
                }

                // Check if there are more results
                match result.continuation_token {
                    Some(token) if !result.entries.is_empty() => {
                        continuation_token = Some(token);
                    }
                    _ => break,
                }
            }
        });

        tokio_stream::wrappers::ReceiverStream::new(rx).boxed()
    }
}

#[cfg(test)]
mod tests {
    // Tests require a KV store implementation, which is provided by aspen-testing
    // Integration tests are in the tests/ directory
}
