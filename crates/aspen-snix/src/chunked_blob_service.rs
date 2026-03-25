//! Chunked blob service wrapping an inner `BlobService`.
//!
//! Blobs above [`INLINE_THRESHOLD`](crate::chunking::INLINE_THRESHOLD) are split
//! into content-defined chunks via FastCDC. Chunks are stored individually in the
//! inner service. A manifest recording chunk hashes and sizes is stored in a KV
//! store (inline for small manifests, as a blob for large ones).
//!
//! Blobs below the threshold pass through to the inner service unchanged.

use std::io;
use std::io::Cursor;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::time::Instant;

use async_trait::async_trait;
use base64::Engine;
use snix_castore::B3Digest;
use snix_castore::blobservice::BlobReader;
use snix_castore::blobservice::BlobService;
use snix_castore::blobservice::BlobWriter;
use snix_castore::proto::stat_blob_response::ChunkMeta;
use tokio::io::AsyncWrite;
use tracing::debug;
use tracing::info;

use crate::chunking::INLINE_THRESHOLD;
use crate::chunking::{self};
use crate::manifest::MAX_INLINE_MANIFEST_SIZE;
use crate::manifest::Manifest;
use crate::manifest::ManifestEntry;

/// KV key prefix for chunk manifests.
const MANIFEST_KEY_PREFIX: &str = "snix:manifest:";

/// ChunkedBlobService wraps an inner BlobService, adding content-defined
/// chunking and cross-blob deduplication.
pub struct ChunkedBlobService<B, K: ?Sized> {
    /// Inner blob service for actual storage.
    inner: B,
    /// KV store for manifest metadata.
    kv: Arc<K>,
}

impl<B: Clone, K: ?Sized> Clone for ChunkedBlobService<B, K> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            kv: Arc::clone(&self.kv),
        }
    }
}

impl<B, K> ChunkedBlobService<B, K> {
    /// Create a new ChunkedBlobService wrapping the given blob service and KV store.
    pub fn new(inner: B, kv: K) -> Self {
        Self {
            inner,
            kv: Arc::new(kv),
        }
    }
}

impl<B, K: ?Sized> ChunkedBlobService<B, K> {
    /// Create from an Arc'd KV store.
    pub fn new_with_arc(inner: B, kv: Arc<K>) -> Self {
        Self { inner, kv }
    }

    /// Build the manifest KV key for a blob digest.
    fn manifest_key(digest: &B3Digest) -> String {
        format!("{}{}", MANIFEST_KEY_PREFIX, hex::encode(digest.as_ref()))
    }
}

impl<B, K> ChunkedBlobService<B, K>
where K: aspen_core::KeyValueStore + Send + Sync + 'static + ?Sized
{
    /// Load a manifest from KV for the given blob digest.
    async fn load_manifest(&self, digest: &B3Digest) -> io::Result<Option<Manifest>> {
        let key = Self::manifest_key(digest);
        let result = match self.kv.read(aspen_core::kv::ReadRequest::new(&key)).await {
            Ok(r) => r,
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(io::Error::other(format!("manifest KV read: {e}"))),
        };

        match result.kv {
            Some(kv) => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&kv.value)
                    .map_err(|e| io::Error::other(format!("manifest base64 decode: {e}")))?;
                let manifest = Manifest::from_bytes(&bytes)
                    .map_err(|e| io::Error::other(format!("manifest postcard decode: {e}")))?;
                Ok(Some(manifest))
            }
            None => Ok(None),
        }
    }
}

#[async_trait]
impl<B, K> BlobService for ChunkedBlobService<B, K>
where
    B: BlobService + Clone + Send + Sync + 'static,
    K: aspen_core::KeyValueStore + Send + Sync + 'static + ?Sized,
{
    async fn has(&self, digest: &B3Digest) -> io::Result<bool> {
        // Check inner service first (covers unchunked blobs and chunk data)
        if self.inner.has(digest).await? {
            return Ok(true);
        }
        // Also check if we have a manifest for this digest (chunked blob)
        Ok(self.load_manifest(digest).await?.is_some())
    }

    async fn open_read(&self, digest: &B3Digest) -> io::Result<Option<Box<dyn BlobReader>>> {
        // Try direct read first (unchunked blob or chunk itself)
        if let Some(reader) = self.inner.open_read(digest).await? {
            // Check if there's a manifest — if so, we need to reassemble
            if let Some(manifest) = self.load_manifest(digest).await? {
                return self.reassemble_chunked(manifest).await;
            }
            return Ok(Some(reader));
        }

        // No direct blob — check for manifest (chunked blob)
        if let Some(manifest) = self.load_manifest(digest).await? {
            return self.reassemble_chunked(manifest).await;
        }

        Ok(None)
    }

    async fn open_write(&self) -> Box<dyn BlobWriter> {
        Box::new(ChunkedBlobWriter {
            inner: self.inner.clone(),
            kv: Arc::clone(&self.kv),
            buffer: Vec::new(),
            is_closed: false,
            digest: None,
        })
    }

    async fn chunks(&self, digest: &B3Digest) -> io::Result<Option<Vec<ChunkMeta>>> {
        if let Some(manifest) = self.load_manifest(digest).await? {
            let chunk_metas = manifest
                .entries
                .iter()
                .map(|e| ChunkMeta {
                    digest: e.hash.to_vec().into(),
                    size: e.size as u64,
                })
                .collect();
            return Ok(Some(chunk_metas));
        }

        // No manifest — check if blob exists at all
        if self.inner.has(digest).await? {
            // Unchunked blob, return empty vec (matches existing behavior)
            return Ok(Some(vec![]));
        }

        Ok(None)
    }
}

impl<B, K> ChunkedBlobService<B, K>
where
    B: BlobService + Send + Sync + 'static,
    K: Send + Sync + 'static + ?Sized,
{
    /// Reassemble a chunked blob from its manifest.
    async fn reassemble_chunked(&self, manifest: Manifest) -> io::Result<Option<Box<dyn BlobReader>>> {
        let mut assembled = Vec::with_capacity(manifest.total_size as usize);

        for entry in &manifest.entries {
            let chunk_digest = B3Digest::from(&entry.hash);
            let reader = self.inner.open_read(&chunk_digest).await?.ok_or_else(|| {
                io::Error::other(format!("missing chunk {} during reassembly", hex::encode(entry.hash)))
            })?;

            let mut buf = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut tokio::io::BufReader::new(TokioReadAdapter(reader)), &mut buf)
                .await?;

            // Verify chunk hash
            let actual_hash = blake3::hash(&buf);
            if *actual_hash.as_bytes() != entry.hash {
                return Err(io::Error::other(format!(
                    "chunk hash mismatch: expected {}, got {}",
                    hex::encode(entry.hash),
                    actual_hash
                )));
            }

            assembled.extend_from_slice(&buf);
        }

        Ok(Some(Box::new(Cursor::new(assembled))))
    }
}

/// Adapter: Box<dyn BlobReader> → tokio::io::AsyncRead
struct TokioReadAdapter(Box<dyn BlobReader>);

impl tokio::io::AsyncRead for TokioReadAdapter {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut *self.0).poll_read(cx, buf)
    }
}

/// BlobWriter that chunks on close if the blob exceeds the inline threshold.
struct ChunkedBlobWriter<B, K: ?Sized> {
    inner: B,
    kv: Arc<K>,
    buffer: Vec<u8>,
    is_closed: bool,
    digest: Option<B3Digest>,
}

impl<B, K: ?Sized> Unpin for ChunkedBlobWriter<B, K> {}

impl<B, K> AsyncWrite for ChunkedBlobWriter<B, K>
where
    B: Send + Sync + 'static,
    K: Send + Sync + 'static + ?Sized,
{
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
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

// Also implement n0_future::io::AsyncWrite for BlobWriter trait compat
impl<B, K> n0_future::io::AsyncWrite for ChunkedBlobWriter<B, K>
where
    B: Send + Sync + 'static,
    K: Send + Sync + 'static + ?Sized,
{
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.buffer.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[async_trait]
impl<B, K> BlobWriter for ChunkedBlobWriter<B, K>
where
    B: BlobService + Send + Sync + 'static,
    K: aspen_core::KeyValueStore + Send + Sync + 'static + ?Sized,
{
    async fn close(&mut self) -> io::Result<B3Digest> {
        if self.is_closed {
            return self.digest.ok_or_else(|| io::Error::other("writer closed without digest"));
        }
        self.is_closed = true;

        let data = std::mem::take(&mut self.buffer);
        let blob_hash = blake3::hash(&data);
        let blob_digest = B3Digest::from(blob_hash.as_bytes());

        if (data.len() as u64) <= INLINE_THRESHOLD {
            // Small blob — store directly, no chunking
            let mut writer = self.inner.open_write().await;
            tokio::io::AsyncWriteExt::write_all(&mut writer, &data).await?;
            let digest = writer.close().await?;
            self.digest = Some(digest);
            debug!(size = data.len(), "stored unchunked blob");
            return Ok(digest);
        }

        // Large blob — chunk it
        let start = Instant::now();
        let chunks = chunking::chunk_blob(&data);
        let chunk_count = chunks.len();
        let mut dedup_count = 0u32;

        let mut manifest_entries = Vec::with_capacity(chunks.len());

        for chunk in &chunks {
            let chunk_digest = B3Digest::from(chunk.hash.as_bytes());

            // Dedup: check if chunk already exists
            if self.inner.has(&chunk_digest).await? {
                dedup_count += 1;
            } else {
                // Store the new chunk
                let chunk_data = &data[chunk.offset as usize..(chunk.offset + chunk.size as u64) as usize];
                let mut writer = self.inner.open_write().await;
                tokio::io::AsyncWriteExt::write_all(&mut writer, chunk_data).await?;
                writer.close().await?;
            }

            manifest_entries.push(ManifestEntry {
                hash: *chunk.hash.as_bytes(),
                size: chunk.size,
            });
        }

        let manifest = Manifest::new(manifest_entries);

        // Store manifest
        let manifest_key = format!("{}{}", MANIFEST_KEY_PREFIX, hex::encode(blob_digest.as_ref()));
        let manifest_bytes = manifest.to_bytes().map_err(|e| io::Error::other(format!("manifest serialize: {e}")))?;

        if manifest_bytes.len() <= MAX_INLINE_MANIFEST_SIZE {
            // Inline in KV
            let value = base64::engine::general_purpose::STANDARD.encode(&manifest_bytes);
            self.kv
                .write(aspen_core::kv::WriteRequest {
                    command: aspen_core::kv::WriteCommand::Set {
                        key: manifest_key,
                        value,
                    },
                })
                .await
                .map_err(|e| io::Error::other(format!("manifest KV write: {e}")))?;
        } else {
            // Large manifest — store as blob, put a pointer in KV
            let mut mw = self.inner.open_write().await;
            tokio::io::AsyncWriteExt::write_all(&mut mw, &manifest_bytes).await?;
            let manifest_digest = mw.close().await?;
            let pointer = format!("blob:{}", hex::encode(manifest_digest.as_ref()));
            self.kv
                .write(aspen_core::kv::WriteRequest {
                    command: aspen_core::kv::WriteCommand::Set {
                        key: manifest_key,
                        value: pointer,
                    },
                })
                .await
                .map_err(|e| io::Error::other(format!("manifest pointer KV write: {e}")))?;
        }

        // Also store the whole blob in the inner service for direct access
        // (existing readers that don't know about chunking still work)
        let mut whole_writer = self.inner.open_write().await;
        tokio::io::AsyncWriteExt::write_all(&mut whole_writer, &data).await?;
        whole_writer.close().await?;

        let elapsed = start.elapsed();
        info!(
            size = data.len(),
            chunks = chunk_count,
            deduped = dedup_count,
            elapsed_ms = elapsed.as_millis() as u64,
            "stored chunked blob"
        );

        self.digest = Some(blob_digest);
        Ok(blob_digest)
    }
}

#[cfg(test)]
mod tests {
    use aspen_blob::InMemoryBlobStore;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;
    use crate::blob_service::IrohBlobService;

    type TestService = ChunkedBlobService<IrohBlobService<InMemoryBlobStore>, DeterministicKeyValueStore>;

    fn make_service() -> TestService {
        let store = InMemoryBlobStore::new();
        let inner = IrohBlobService::new(store);
        let kv = DeterministicKeyValueStore::new(); // returns Arc<Self>
        ChunkedBlobService::new_with_arc(inner, kv)
    }

    /// Helper to write data through the service and return the digest.
    async fn write_blob(svc: &TestService, data: &[u8]) -> B3Digest {
        let mut w = BlobService::open_write(svc).await;
        BlobWriter::close(&mut *w).await.unwrap();
        // Re-do with actual data
        let mut w = BlobService::open_write(svc).await;
        tokio::io::AsyncWriteExt::write_all(&mut w, data).await.unwrap();
        BlobWriter::close(&mut *w).await.unwrap()
    }

    /// Helper to read a blob back.
    async fn read_blob(svc: &TestService, digest: &B3Digest) -> Vec<u8> {
        let reader = BlobService::open_read(svc, digest).await.unwrap().unwrap();
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut tokio::io::BufReader::new(TokioReadAdapter(reader)), &mut buf)
            .await
            .unwrap();
        buf
    }

    #[tokio::test]
    async fn roundtrip_small_blob_no_chunking() {
        let svc = make_service();
        let data = b"hello small world";

        let mut w = BlobService::open_write(&svc).await;
        tokio::io::AsyncWriteExt::write_all(&mut w, data.as_slice()).await.unwrap();
        let digest = BlobWriter::close(&mut *w).await.unwrap();

        assert!(BlobService::has(&svc, &digest).await.unwrap());
        let buf = read_blob(&svc, &digest).await;
        assert_eq!(buf, data);

        // No manifest for small blobs — chunks() returns empty vec
        let chunks = BlobService::chunks(&svc, &digest).await.unwrap().unwrap();
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn roundtrip_large_blob_chunked() {
        let svc = make_service();
        // 512 KiB of pseudo-random data (above INLINE_THRESHOLD)
        let data: Vec<u8> = (0..524_288u32).map(|i| (i.wrapping_mul(2654435761)) as u8).collect();

        let mut w = BlobService::open_write(&svc).await;
        tokio::io::AsyncWriteExt::write_all(&mut w, &data).await.unwrap();
        let digest = BlobWriter::close(&mut *w).await.unwrap();

        assert!(BlobService::has(&svc, &digest).await.unwrap());
        let buf = read_blob(&svc, &digest).await;
        assert_eq!(buf, data);

        // chunks() returns real metadata
        let chunks = BlobService::chunks(&svc, &digest).await.unwrap().unwrap();
        assert!(!chunks.is_empty());
        let total: u64 = chunks.iter().map(|c| c.size).sum();
        assert_eq!(total, data.len() as u64);
    }

    #[tokio::test]
    async fn dedup_across_two_blobs_with_shared_content() {
        let svc = make_service();

        // Two blobs that share a large prefix
        let shared: Vec<u8> = (0..400_000u32).map(|i| (i.wrapping_mul(2654435761)) as u8).collect();

        let mut blob1 = shared.clone();
        blob1.extend_from_slice(&[1u8; 200_000]);

        let mut blob2 = shared.clone();
        blob2.extend_from_slice(&[2u8; 200_000]);

        let mut w1 = BlobService::open_write(&svc).await;
        tokio::io::AsyncWriteExt::write_all(&mut w1, &blob1).await.unwrap();
        let d1 = BlobWriter::close(&mut *w1).await.unwrap();

        let mut w2 = BlobService::open_write(&svc).await;
        tokio::io::AsyncWriteExt::write_all(&mut w2, &blob2).await.unwrap();
        let d2 = BlobWriter::close(&mut *w2).await.unwrap();

        // Both blobs readable
        assert_eq!(read_blob(&svc, &d1).await, blob1);
        assert_eq!(read_blob(&svc, &d2).await, blob2);
    }

    #[tokio::test]
    async fn chunks_returns_correct_metadata() {
        let svc = make_service();
        let data: Vec<u8> = (0..524_288u32).map(|i| (i.wrapping_mul(2654435761)) as u8).collect();

        let mut w = BlobService::open_write(&svc).await;
        tokio::io::AsyncWriteExt::write_all(&mut w, &data).await.unwrap();
        let digest = BlobWriter::close(&mut *w).await.unwrap();

        let chunks = BlobService::chunks(&svc, &digest).await.unwrap().unwrap();
        assert!(!chunks.is_empty());

        // Each chunk digest should be 32 bytes
        for c in &chunks {
            assert_eq!(c.digest.len(), 32);
            assert!(c.size > 0);
        }
    }

    #[tokio::test]
    async fn missing_blob_returns_none() {
        let svc = make_service();
        let fake_digest = B3Digest::from(&[0u8; 32]);

        assert!(!BlobService::has(&svc, &fake_digest).await.unwrap());
        assert!(BlobService::open_read(&svc, &fake_digest).await.unwrap().is_none());
        assert!(BlobService::chunks(&svc, &fake_digest).await.unwrap().is_none());
    }
}

#[cfg(test)]
mod proptests {
    use aspen_blob::InMemoryBlobStore;
    use aspen_testing::DeterministicKeyValueStore;
    use proptest::prelude::*;

    use super::*;
    use crate::blob_service::IrohBlobService;

    type TestService = ChunkedBlobService<IrohBlobService<InMemoryBlobStore>, DeterministicKeyValueStore>;

    fn make_service() -> TestService {
        let store = InMemoryBlobStore::new();
        let inner = IrohBlobService::new(store);
        let kv = DeterministicKeyValueStore::new();
        ChunkedBlobService::new_with_arc(inner, kv)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        #[test]
        fn roundtrip_arbitrary_blob(data in proptest::collection::vec(any::<u8>(), 0..600_000)) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let svc = make_service();
                let mut w = BlobService::open_write(&svc).await;
                tokio::io::AsyncWriteExt::write_all(&mut w, &data).await.unwrap();
                let digest = BlobWriter::close(&mut *w).await.unwrap();

                let reader = BlobService::open_read(&svc, &digest).await.unwrap().unwrap();
                let mut buf = Vec::new();
                tokio::io::AsyncReadExt::read_to_end(
                    &mut tokio::io::BufReader::new(TokioReadAdapter(reader)),
                    &mut buf,
                ).await.unwrap();
                prop_assert_eq!(buf, data);
                Ok(())
            }).unwrap();
        }
    }
}
