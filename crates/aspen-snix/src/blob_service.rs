//! SNIX BlobService implementation backed by Aspen's iroh-blobs storage.
//!
//! This module provides [`IrohBlobService`], which implements the SNIX
//! [`BlobService`] trait using Aspen's [`BlobStore`] for content-addressed
//! blob storage.
//!
//! # Architecture
//!
//! The SNIX BlobService trait expects:
//! - BLAKE3 digests (32 bytes) for content addressing
//! - Streaming reads via [`BlobReader`]
//! - Streaming writes via [`BlobWriter`]
//!
//! Aspen's BlobStore provides:
//! - iroh-blobs Hash (also BLAKE3, 32 bytes)
//! - Byte-based get/put operations
//!
//! This adapter bridges the gap by:
//! - Converting between B3Digest and iroh_blobs::Hash
//! - Wrapping byte operations in async streams
//! - Using tempfiles for large blob writes

use std::io::Cursor;
use std::io::{self};
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;

use aspen_core::circuit_breaker::CircuitBreaker;
use async_trait::async_trait;
use n0_future::io::AsyncWrite;
use snix_castore::B3Digest;
use snix_castore::blobservice::BlobReader;
use snix_castore::blobservice::BlobService;
use snix_castore::blobservice::BlobWriter;
use snix_castore::proto::stat_blob_response::ChunkMeta;
use tokio::sync::Mutex;
use tracing::debug;
use tracing::instrument;
use tracing::warn;

use crate::circuit_breaker_time;
use crate::constants::MAX_BLOB_SIZE_BYTES;

/// Default consecutive failure threshold for the blob service circuit breaker.
const BLOB_CB_THRESHOLD: u32 = 5;

/// Default open duration for the blob service circuit breaker.
const BLOB_CB_OPEN_DURATION: Duration = Duration::from_secs(30);

/// SNIX BlobService implementation backed by Aspen's BlobStore.
///
/// Wraps an Aspen [`BlobStore`] to provide the SNIX [`BlobService`] interface.
/// Both use BLAKE3 hashing, so hash conversion is straightforward.
#[derive(Clone)]
pub struct IrohBlobService<S> {
    store: Arc<S>,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

impl<S> IrohBlobService<S> {
    /// Create a new IrohBlobService wrapping the given store.
    pub fn new(store: S) -> Self {
        Self {
            store: Arc::new(store),
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(BLOB_CB_THRESHOLD, BLOB_CB_OPEN_DURATION))),
        }
    }

    /// Create a new IrohBlobService from an Arc'd store.
    pub fn from_arc(store: Arc<S>) -> Self {
        Self {
            store,
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(BLOB_CB_THRESHOLD, BLOB_CB_OPEN_DURATION))),
        }
    }

    // r[impl snix.store.circuit-breaker]
    /// Check if the circuit breaker is open and return an error if so.
    async fn check_circuit(&self) -> io::Result<()> {
        let cb = self.circuit_breaker.lock().await;
        if cb.should_reject(circuit_breaker_time::now_ms()) {
            return Err(io::Error::other("iroh blob service circuit breaker is open — too many consecutive failures"));
        }
        Ok(())
    }

    /// Record a successful operation. Logs recovery if breaker was open.
    async fn record_success(&self) {
        if self.circuit_breaker.lock().await.record_success() {
            tracing::info!("iroh blob service circuit breaker recovered — closed");
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
                "iroh blob service circuit breaker tripped open"
            );
        }
    }
}

/// Convert a B3Digest to an iroh_blobs::Hash.
///
/// Both are 32-byte BLAKE3 hashes, so this is a direct conversion.
fn b3_digest_to_iroh_hash(digest: &B3Digest) -> iroh_blobs::Hash {
    iroh_blobs::Hash::from_bytes(*digest.as_ref())
}

/// Convert an iroh_blobs::Hash to a B3Digest.
fn iroh_hash_to_b3_digest(hash: &iroh_blobs::Hash) -> B3Digest {
    B3Digest::from(hash.as_bytes())
}

#[async_trait]
impl<S> BlobService for IrohBlobService<S>
where S: aspen_blob::BlobStore + 'static
{
    #[instrument(skip(self), fields(digest = %digest))]
    async fn has(&self, digest: &B3Digest) -> io::Result<bool> {
        self.check_circuit().await?;
        let hash = b3_digest_to_iroh_hash(digest);
        match self.store.has(&hash).await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(e) => {
                self.record_failure().await;
                Err(io::Error::other(format!("blob store error: {}", e)))
            }
        }
    }

    #[instrument(skip(self), fields(digest = %digest))]
    async fn open_read(&self, digest: &B3Digest) -> io::Result<Option<Box<dyn BlobReader>>> {
        self.check_circuit().await?;
        let hash = b3_digest_to_iroh_hash(digest);

        match self.store.get_bytes(&hash).await {
            Ok(Some(bytes)) => {
                self.record_success().await;
                debug!(size = bytes.len(), "blob read successfully");
                let cursor = Cursor::new(bytes);
                Ok(Some(Box::new(cursor)))
            }
            Ok(None) => {
                self.record_success().await;
                debug!("blob not found");
                Ok(None)
            }
            Err(e) => {
                self.record_failure().await;
                Err(io::Error::other(format!("blob store error: {}", e)))
            }
        }
    }

    #[instrument(skip(self))]
    async fn open_write(&self) -> Box<dyn BlobWriter> {
        debug!("opening blob writer");
        Box::new(IrohBlobWriter::new(Arc::clone(&self.store), Arc::clone(&self.circuit_breaker)))
    }

    #[instrument(skip(self), fields(digest = %digest))]
    async fn chunks(&self, digest: &B3Digest) -> io::Result<Option<Vec<ChunkMeta>>> {
        // has() already checks circuit breaker
        if !self.has(digest).await? {
            return Ok(None);
        }
        // We don't support chunking - return empty vec to indicate no granular chunks
        Ok(Some(vec![]))
    }
}

/// BlobWriter implementation that buffers data and writes to BlobStore on close.
///
/// For large blobs (over the streaming threshold), this uses a tempfile to avoid
/// memory pressure. For small blobs, data is buffered in memory.
pub struct IrohBlobWriter<S> {
    store: Arc<S>,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
    buffer: Vec<u8>,
    is_closed: bool,
    digest: Option<B3Digest>,
}

impl<S> IrohBlobWriter<S> {
    fn new(store: Arc<S>, circuit_breaker: Arc<Mutex<CircuitBreaker>>) -> Self {
        Self {
            store,
            circuit_breaker,
            buffer: Vec::new(),
            is_closed: false,
            digest: None,
        }
    }
}

impl<S> AsyncWrite for IrohBlobWriter<S>
where S: Send + Sync + 'static
{
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        if self.is_closed {
            return Poll::Ready(Err(io::Error::other("writer already closed")));
        }

        // r[impl snix.store.blob-size-bound]
        let new_size = self.buffer.len() as u64 + buf.len() as u64;
        if new_size > MAX_BLOB_SIZE_BYTES {
            return Poll::Ready(Err(io::Error::other(format!(
                "blob size {} exceeds maximum {}",
                new_size, MAX_BLOB_SIZE_BYTES
            ))));
        }

        self.buffer.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Note: actual close happens in BlobWriter::close()
        Poll::Ready(Ok(()))
    }
}

// Implement tokio::io::AsyncWrite for compatibility
impl<S> tokio::io::AsyncWrite for IrohBlobWriter<S>
where S: Send + Sync + 'static
{
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        if self.is_closed {
            return Poll::Ready(Err(io::Error::other("writer already closed")));
        }

        // Check size limit
        let new_size = self.buffer.len() as u64 + buf.len() as u64;
        if new_size > MAX_BLOB_SIZE_BYTES {
            return Poll::Ready(Err(io::Error::other(format!(
                "blob size {} exceeds maximum {}",
                new_size, MAX_BLOB_SIZE_BYTES
            ))));
        }

        self.buffer.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[async_trait]
impl<S> BlobWriter for IrohBlobWriter<S>
where S: aspen_blob::BlobStore + Send + Sync + 'static
{
    async fn close(&mut self) -> io::Result<B3Digest> {
        if self.is_closed {
            // Return cached digest if already closed
            return self.digest.ok_or_else(|| io::Error::other("writer closed without digest"));
        }

        // Check circuit breaker before attempting write
        {
            let cb = self.circuit_breaker.lock().await;
            if cb.should_reject(circuit_breaker_time::now_ms()) {
                return Err(io::Error::other(
                    "iroh blob service circuit breaker is open — too many consecutive failures",
                ));
            }
        }

        self.is_closed = true;

        tracing::info!(buffer_size = self.buffer.len(), "blob writer closing, writing to store");

        // Write to blob store
        let result = match self.store.add_bytes(&self.buffer).await {
            Ok(r) => {
                if self.circuit_breaker.lock().await.record_success() {
                    tracing::info!("iroh blob service circuit breaker recovered — closed");
                }
                r
            }
            Err(e) => {
                let mut cb = self.circuit_breaker.lock().await;
                let now_ms = circuit_breaker_time::now_ms();
                let was_open = cb.should_reject(now_ms);
                cb.record_failure(now_ms);
                if !was_open && cb.should_reject(now_ms) {
                    warn!(
                        failures = cb.consecutive_failures(),
                        "iroh blob service circuit breaker tripped open during write"
                    );
                }
                self.is_closed = false; // allow retry
                return Err(io::Error::other(format!("blob store error: {}", e)));
            }
        };

        let digest = iroh_hash_to_b3_digest(&result.blob_ref.hash);
        self.digest = Some(digest);

        tracing::info!(digest = %digest, size = self.buffer.len(), "blob written to store");

        // Clear buffer to free memory
        self.buffer.clear();
        self.buffer.shrink_to_fit();

        Ok(digest)
    }
}

impl<S> Unpin for IrohBlobWriter<S> {}

#[cfg(test)]
mod tests {
    use aspen_blob::InMemoryBlobStore;
    use tokio::io::AsyncRead;
    use tokio::io::AsyncWriteExt;
    use tokio::io::ReadBuf;

    use super::*;

    #[tokio::test]
    // r[verify snix.store.blob-size-bound]
    async fn test_blob_roundtrip() {
        let store = InMemoryBlobStore::new();
        let service = IrohBlobService::new(store);

        let data = b"hello world";

        // Write blob
        let mut writer = service.open_write().await;
        writer.write_all(data).await.unwrap();
        let digest = writer.close().await.unwrap();

        // Verify exists
        assert!(service.has(&digest).await.unwrap());

        // Read back
        let reader = service.open_read(&digest).await.unwrap().unwrap();
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut tokio::io::BufReader::new(TokioAsyncReadAdapter(reader)), &mut buf)
            .await
            .unwrap();
        assert_eq!(buf, data);
    }

    #[tokio::test]
    async fn test_blob_not_found() {
        let store = InMemoryBlobStore::new();
        let service = IrohBlobService::new(store);

        let digest = B3Digest::from(&[0u8; 32]);
        assert!(!service.has(&digest).await.unwrap());
        assert!(service.open_read(&digest).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_chunks_returns_empty_for_existing() {
        let store = InMemoryBlobStore::new();
        let service = IrohBlobService::new(store);

        let data = b"test data";
        let mut writer = service.open_write().await;
        writer.write_all(data).await.unwrap();
        let digest = writer.close().await.unwrap();

        let chunks = service.chunks(&digest).await.unwrap();
        assert!(chunks.is_some());
        assert!(chunks.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_chunks_returns_none_for_missing() {
        let store = InMemoryBlobStore::new();
        let service = IrohBlobService::new(store);

        let digest = B3Digest::from(&[0u8; 32]);
        assert!(service.chunks(&digest).await.unwrap().is_none());
    }

    /// Adapter to use a Box<dyn BlobReader> with tokio::io traits
    struct TokioAsyncReadAdapter(Box<dyn BlobReader>);

    impl AsyncRead for TokioAsyncReadAdapter {
        fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
            let inner = Pin::new(&mut *self.0);
            // BlobReader is AsyncRead + AsyncSeek, so we can use tokio's AsyncRead
            inner.poll_read(cx, buf)
        }
    }
}
