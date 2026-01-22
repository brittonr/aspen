//! Streaming NAR handler for large file transfers.
//!
//! Implements backpressure-aware streaming from BlobStore to h3 response.

use std::io::SeekFrom;
use std::sync::Arc;
use std::time::Instant;

use aspen_blob::BlobStore;
use aspen_cache::CacheIndex;
use bytes::Bytes;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use crate::config::NixCacheGatewayConfig;
use crate::constants::MAX_STREAM_DURATION;
use crate::endpoints::nar::NarDownload;
use crate::endpoints::nar::extract_blob_hash;
use crate::endpoints::nar::prepare_nar_download;
use crate::error::NixCacheError;
use crate::error::Result;
use crate::signing::NarinfoSigner;

/// Statistics from a streaming operation.
#[derive(Debug, Clone)]
pub struct StreamStats {
    /// Total bytes sent.
    pub bytes_sent: u64,
    /// Number of chunks sent.
    pub chunks_sent: u32,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Whether this was a partial (range) request.
    pub was_partial: bool,
}

/// Handler for streaming NAR files.
///
/// Uses the BlobStore's `reader()` method to stream content without
/// loading the entire blob into memory.
pub struct NarStreamingHandler<I, B>
where
    I: CacheIndex,
    B: BlobStore,
{
    config: Arc<NixCacheGatewayConfig>,
    cache_index: Arc<I>,
    blob_store: Arc<B>,
    signer: Option<NarinfoSigner>,
}

impl<I, B> NarStreamingHandler<I, B>
where
    I: CacheIndex + Send + Sync + 'static,
    B: BlobStore + Send + Sync + 'static,
{
    /// Create a new streaming handler.
    pub fn new(
        config: NixCacheGatewayConfig,
        cache_index: Arc<I>,
        blob_store: Arc<B>,
        signer: Option<NarinfoSigner>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            cache_index,
            blob_store,
            signer,
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &NixCacheGatewayConfig {
        &self.config
    }

    /// Get the signer (if configured).
    pub fn signer(&self) -> Option<&NarinfoSigner> {
        self.signer.as_ref()
    }

    /// Get the cache index.
    pub fn cache_index(&self) -> &I {
        &self.cache_index
    }

    /// Get the blob store.
    pub fn blob_store(&self) -> &B {
        &self.blob_store
    }

    /// Prepare a NAR download from the request path.
    #[instrument(skip(self, range_header), fields(path = %path))]
    pub async fn prepare_download(&self, path: &str, range_header: Option<&str>) -> Result<NarDownload> {
        let blob_hash =
            extract_blob_hash(path).ok_or_else(|| NixCacheError::InvalidStoreHash { hash: path.to_string() })?;

        prepare_nar_download(blob_hash, range_header, self.cache_index.as_ref(), self.blob_store.as_ref()).await
    }

    /// Stream a NAR file to a callback.
    ///
    /// The callback receives chunks of data and should return Ok(()) to continue
    /// or an error to abort. This design allows integration with h3's backpressure.
    #[instrument(skip(self, download, send_chunk), fields(
        blob_hash = %download.blob_hash,
        content_length = download.content_length,
        range = ?download.range
    ))]
    pub async fn stream_nar<F, Fut>(&self, download: &NarDownload, mut send_chunk: F) -> Result<StreamStats>
    where
        F: FnMut(Bytes) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let start_time = Instant::now();
        let chunk_size = self.config.nar_chunk_size_bytes as usize;

        // Get streaming reader
        let mut reader = self
            .blob_store
            .reader(&download.blob_hash)
            .await
            .map_err(|e| NixCacheError::BlobStore { message: e.to_string() })?
            .ok_or_else(|| NixCacheError::BlobNotAvailable {
                blob_hash: download.blob_hash.to_string(),
            })?;

        // Seek to range start if partial
        let (transfer_start, transfer_end) = match download.range {
            Some((start, end)) => {
                reader.seek(SeekFrom::Start(start)).await.map_err(|e| NixCacheError::BlobStore {
                    message: format!("seek error: {}", e),
                })?;
                (start, end)
            }
            None => (0, download.content_length - 1),
        };

        let transfer_size = transfer_end - transfer_start + 1;
        let mut bytes_sent: u64 = 0;
        let mut chunks_sent: u32 = 0;
        let mut buf = vec![0u8; chunk_size];

        while bytes_sent < transfer_size {
            // Check timeout
            if start_time.elapsed() > MAX_STREAM_DURATION {
                return Err(NixCacheError::Timeout {
                    elapsed_ms: start_time.elapsed().as_millis() as u64,
                });
            }

            // Calculate how much to read
            let remaining = (transfer_size - bytes_sent) as usize;
            let read_size = remaining.min(chunk_size);

            // Read chunk from blob store
            let bytes_read = reader.read(&mut buf[..read_size]).await.map_err(|e| NixCacheError::BlobStore {
                message: format!("read error: {}", e),
            })?;

            if bytes_read == 0 {
                // Unexpected EOF
                warn!(bytes_sent, transfer_size, "unexpected EOF during streaming");
                break;
            }

            // Send chunk via callback (this is where h3 backpressure happens)
            let chunk = Bytes::copy_from_slice(&buf[..bytes_read]);
            send_chunk(chunk).await?;

            bytes_sent += bytes_read as u64;
            chunks_sent += 1;

            // Periodic logging for large transfers
            if chunks_sent.is_multiple_of(1000) {
                debug!(
                    bytes_sent,
                    chunks_sent,
                    progress_percent = (bytes_sent * 100 / transfer_size),
                    "streaming progress"
                );
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        info!(bytes_sent, chunks_sent, duration_ms, partial = download.is_partial(), "NAR streaming complete");

        Ok(StreamStats {
            bytes_sent,
            chunks_sent,
            duration_ms,
            was_partial: download.is_partial(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::RwLock;

    use aspen_blob::InMemoryBlobStore;
    use aspen_cache::CacheEntry;

    use super::*;

    /// Mock cache index for testing.
    struct MockCacheIndex {
        entries: RwLock<std::collections::HashMap<String, CacheEntry>>,
    }

    impl MockCacheIndex {
        fn new() -> Self {
            Self {
                entries: RwLock::new(std::collections::HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl CacheIndex for MockCacheIndex {
        async fn get(&self, store_hash: &str) -> aspen_cache::Result<Option<CacheEntry>> {
            let entries = self.entries.read().unwrap();
            Ok(entries.get(store_hash).cloned())
        }

        async fn put(&self, entry: CacheEntry) -> aspen_cache::Result<()> {
            let mut entries = self.entries.write().unwrap();
            entries.insert(entry.store_hash.clone(), entry);
            Ok(())
        }

        async fn exists(&self, store_hash: &str) -> aspen_cache::Result<bool> {
            let entries = self.entries.read().unwrap();
            Ok(entries.contains_key(store_hash))
        }

        async fn stats(&self) -> aspen_cache::Result<aspen_cache::CacheStats> {
            Ok(aspen_cache::CacheStats::default())
        }
    }

    #[tokio::test]
    async fn test_streaming_handler_creation() {
        let blob_store = Arc::new(InMemoryBlobStore::new());
        let cache_index = Arc::new(MockCacheIndex::new());
        let config = NixCacheGatewayConfig::default();

        let handler = NarStreamingHandler::new(config, cache_index, blob_store, None);

        assert_eq!(handler.config().priority, 30);
        assert!(handler.signer().is_none());
    }
}
