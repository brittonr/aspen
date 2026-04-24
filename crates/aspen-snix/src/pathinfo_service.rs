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
use std::time::Duration;

use aspen_core::circuit_breaker::CircuitBreaker;
use async_trait::async_trait;
use base64::Engine;
use n0_future::StreamExt;
use prost::Message;
use snix_store::pathinfoservice::Error;
use snix_store::pathinfoservice::PathInfo;
use snix_store::pathinfoservice::PathInfoService;
use tokio::sync::Mutex;
use tracing::debug;
use tracing::instrument;
use tracing::warn;

use crate::BoxStream;
use crate::circuit_breaker_time;
use crate::constants::PATHINFO_KEY_PREFIX;
use crate::constants::STORE_PATH_DIGEST_LENGTH;

/// Default consecutive failure threshold for the pathinfo service circuit breaker.
const PI_CB_THRESHOLD: u32 = 5;

/// Default open duration for the pathinfo service circuit breaker.
const PI_CB_OPEN_DURATION: Duration = Duration::from_secs(30);

/// SNIX PathInfoService implementation backed by Aspen's Raft KV store.
///
/// Stores PathInfo metadata as protobuf-encoded values, keyed by the 20-byte
/// Nix store path digest. Provides linearizable access to store path metadata
/// across the cluster.
pub struct RaftPathInfoService<K: ?Sized> {
    kv: Arc<K>,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

// Manual Clone: Arc is always cloneable regardless of K
impl<K: ?Sized> Clone for RaftPathInfoService<K> {
    fn clone(&self) -> Self {
        Self {
            kv: Arc::clone(&self.kv),
            circuit_breaker: Arc::clone(&self.circuit_breaker),
        }
    }
}

impl<K> RaftPathInfoService<K> {
    /// Create a new RaftPathInfoService with the given KV store.
    pub fn new(kv: K) -> Self {
        Self {
            kv: Arc::new(kv),
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(PI_CB_THRESHOLD, PI_CB_OPEN_DURATION))),
        }
    }
}

impl<K: ?Sized> RaftPathInfoService<K> {
    /// Create a new RaftPathInfoService from an Arc'd KV store.
    pub fn from_arc(kv: Arc<K>) -> Self {
        Self {
            kv,
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(PI_CB_THRESHOLD, PI_CB_OPEN_DURATION))),
        }
    }

    // r[impl snix.store.circuit-breaker]
    /// Check if the circuit breaker is open and return an error if so.
    async fn check_circuit(&self) -> Result<(), Error> {
        let cb = self.circuit_breaker.lock().await;
        if cb.should_reject(circuit_breaker_time::now_ms()) {
            return Err(Box::new(std::io::Error::other(
                "pathinfo service circuit breaker is open — too many consecutive failures",
            )));
        }
        Ok(())
    }

    /// Record a successful operation. Logs recovery if breaker was open.
    async fn record_success(&self) {
        if self.circuit_breaker.lock().await.record_success() {
            tracing::info!("pathinfo service circuit breaker recovered — closed");
        }
    }

    /// Record a failed operation. Logs a warning when the breaker trips open.
    async fn record_failure(&self) {
        let mut cb = self.circuit_breaker.lock().await;
        let now_ms = circuit_breaker_time::now_ms();
        let was_open = cb.should_reject(now_ms);
        cb.record_failure(now_ms);
        if !was_open && cb.should_reject(now_ms) {
            warn!(
                failures = cb.consecutive_failures(),
                open_duration_secs = cb.open_duration().as_secs(),
                "pathinfo service circuit breaker tripped open"
            );
        }
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
where K: aspen_core::KeyValueStore + Send + Sync + 'static + ?Sized
{
    #[instrument(skip(self), fields(digest = hex::encode(digest)))]
    async fn get(&self, digest: [u8; 20]) -> Result<Option<PathInfo>, Error> {
        self.check_circuit().await?;
        let key = Self::make_key(&digest);

        let result = match self.kv.read(aspen_core::kv::ReadRequest::new(&key)).await {
            Ok(result) => {
                self.record_success().await;
                result
            }
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => {
                self.record_success().await;
                debug!("path info not found");
                return Ok(None);
            }
            Err(e) => {
                self.record_failure().await;
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
        self.check_circuit().await?;

        tracing::info!(store_path = %path_info.store_path, "pathinfo put called");

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
        tracing::info!(key = %key, value_len = value.len(), "writing pathinfo to KV");
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
                return Err(Box::new(std::io::Error::other(format!("KV write error: {}", e))));
            }
        }

        tracing::info!(store_path = %path_info.store_path, "pathinfo stored successfully");
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
