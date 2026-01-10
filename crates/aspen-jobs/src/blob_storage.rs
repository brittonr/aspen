//! Large payload storage using content-addressed blobs.
//!
//! This module provides blob storage capabilities for job payloads,
//! simulating iroh-blobs functionality with content-addressed storage.

use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

use crate::error::JobError;
use crate::error::Result;
use crate::job::JobId;
use crate::job::JobSpec;
use crate::manager::JobManager;

/// Threshold for storing payloads in blobs (1 MB).
pub const BLOB_THRESHOLD_BYTES: usize = 1024 * 1024;

/// Maximum blob size (100 MB).
pub const MAX_BLOB_SIZE_BYTES: usize = 100 * 1024 * 1024;

/// Maximum decompressed size (100 MB).
/// This limit prevents compression bomb attacks where a small compressed
/// payload expands to exhaust memory during decompression.
pub const MAX_DECOMPRESSED_SIZE: usize = 100 * 1024 * 1024;

/// Content hash for blob identification.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlobHash(String);

impl BlobHash {
    /// Create from hex string.
    pub fn from_hex(hex: &str) -> Result<Self> {
        Ok(BlobHash(hex.to_string()))
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> &str {
        &self.0
    }

    /// Compute hash of data.
    pub fn from_data(data: &[u8]) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();
        BlobHash(format!("{:016x}", hash))
    }
}

/// Blob-backed job payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobPayload {
    /// Hash of the blob containing the actual payload.
    pub blob_hash: BlobHash,
    /// Format of the blob data.
    pub format: PayloadFormat,
    /// Size in bytes.
    pub size: u64,
    /// Optional metadata.
    pub metadata: serde_json::Value,
}

/// Format of blob payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PayloadFormat {
    /// JSON data.
    Json,
    /// MessagePack binary format.
    MessagePack,
    /// Raw binary data.
    Binary,
    /// Compressed JSON (gzip).
    CompressedJson,
}

/// In-memory blob store (simulating iroh-blobs).
#[derive(Clone)]
struct BlobStore {
    blobs: Arc<tokio::sync::RwLock<HashMap<BlobHash, Bytes>>>,
}

impl BlobStore {
    fn new() -> Self {
        Self {
            blobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    async fn store(&self, data: Bytes) -> BlobHash {
        let hash = BlobHash::from_data(&data);
        let mut blobs = self.blobs.write().await;
        blobs.insert(hash.clone(), data);
        hash
    }

    async fn fetch(&self, hash: &BlobHash) -> Option<Bytes> {
        let blobs = self.blobs.read().await;
        blobs.get(hash).cloned()
    }
}

/// Job blob storage manager.
pub struct JobBlobStorage {
    /// Blob store.
    store: BlobStore,
    /// Compression threshold (bytes).
    compression_threshold: usize,
    /// Enable automatic compression.
    auto_compress: bool,
}

impl JobBlobStorage {
    /// Create a new blob storage manager.
    pub async fn new() -> Result<Self> {
        Ok(Self {
            store: BlobStore::new(),
            compression_threshold: 10 * 1024, // 10 KB
            auto_compress: true,
        })
    }

    /// Store a large payload in blob storage.
    pub async fn store_payload(&self, payload: &serde_json::Value, job_id: &str) -> Result<BlobPayload> {
        // Serialize payload
        let data = serde_json::to_vec(payload).map_err(|e| JobError::SerializationError { source: e })?;

        let size = data.len();
        if size > MAX_BLOB_SIZE_BYTES {
            return Err(JobError::InvalidJobSpec {
                reason: format!("Payload too large: {} bytes (max: {} bytes)", size, MAX_BLOB_SIZE_BYTES),
            });
        }

        info!(
            job_id = %job_id,
            size_bytes = size,
            "storing large payload in blob storage"
        );

        // Determine format and potentially compress
        let (blob_data, format) = if self.auto_compress && size > self.compression_threshold {
            let compressed = self.compress_data(&data)?;
            let compression_ratio = compressed.len() as f32 / size as f32;

            info!(
                job_id = %job_id,
                original_size = size,
                compressed_size = compressed.len(),
                compression_ratio = format!("{:.2}", compression_ratio),
                "compressed payload"
            );

            (compressed, PayloadFormat::CompressedJson)
        } else {
            (data, PayloadFormat::Json)
        };

        // Store in blob store
        let hash = self.store.store(Bytes::from(blob_data)).await;

        Ok(BlobPayload {
            blob_hash: hash,
            format,
            size: size as u64,
            metadata: serde_json::json!({
                "job_id": job_id,
                "stored_at": chrono::Utc::now().to_rfc3339(),
            }),
        })
    }

    /// Retrieve a payload from blob storage.
    pub async fn retrieve_payload(&self, blob_payload: &BlobPayload) -> Result<serde_json::Value> {
        info!(
            blob_hash = %blob_payload.blob_hash.to_hex(),
            format = ?blob_payload.format,
            "retrieving payload from blob storage"
        );

        // Fetch blob data
        let data = self.store.fetch(&blob_payload.blob_hash).await.ok_or_else(|| JobError::JobNotFound {
            id: blob_payload.blob_hash.to_hex().to_string(),
        })?;

        // Decompress if needed
        let json_data = match blob_payload.format {
            PayloadFormat::CompressedJson => self.decompress_data(&data)?,
            PayloadFormat::Json => data.to_vec(),
            _ => {
                return Err(JobError::InvalidJobSpec {
                    reason: format!("Unsupported payload format: {:?}", blob_payload.format),
                });
            }
        };

        // Deserialize
        serde_json::from_slice(&json_data).map_err(|e| JobError::SerializationError { source: e })
    }

    /// Compress data using gzip.
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use std::io::Write;

        use flate2::Compression;
        use flate2::write::GzEncoder;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).map_err(|e| JobError::SerializationError {
            source: serde_json::Error::io(e),
        })?;

        encoder.finish().map_err(|e| JobError::SerializationError {
            source: serde_json::Error::io(e),
        })
    }

    /// Decompress gzip data with bounded output size.
    ///
    /// This function prevents compression bomb attacks by limiting the maximum
    /// decompressed size to `MAX_DECOMPRESSED_SIZE`. If the decompressed data
    /// exceeds this limit, an error is returned immediately.
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use std::io::Read;

        use flate2::read::GzDecoder;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        let mut buffer = [0u8; 8192];
        let mut total_read = 0usize;

        loop {
            let bytes_read = decoder.read(&mut buffer).map_err(|e| JobError::SerializationError {
                source: serde_json::Error::io(e),
            })?;

            if bytes_read == 0 {
                break;
            }

            total_read = total_read.saturating_add(bytes_read);
            if total_read > MAX_DECOMPRESSED_SIZE {
                return Err(JobError::DecompressionTooLarge {
                    max: MAX_DECOMPRESSED_SIZE,
                });
            }

            decompressed.extend_from_slice(&buffer[..bytes_read]);
        }

        Ok(decompressed)
    }

    /// Check if a payload should be stored as a blob.
    pub fn should_use_blob(payload: &serde_json::Value) -> bool {
        let size = serde_json::to_vec(payload).map(|v| v.len()).unwrap_or(0);

        size >= BLOB_THRESHOLD_BYTES
    }

    /// Create a blob reference for a job.
    pub fn create_blob_reference(blob_payload: &BlobPayload) -> serde_json::Value {
        serde_json::json!({
            "__blob": true,
            "hash": blob_payload.blob_hash.to_hex(),
            "format": blob_payload.format,
            "size": blob_payload.size,
            "metadata": blob_payload.metadata,
        })
    }

    /// Check if a value is a blob reference.
    pub fn is_blob_reference(value: &serde_json::Value) -> bool {
        value.get("__blob").and_then(|v| v.as_bool()).unwrap_or(false)
    }

    /// Extract blob payload from reference.
    pub fn extract_blob_payload(value: &serde_json::Value) -> Result<BlobPayload> {
        if !Self::is_blob_reference(value) {
            return Err(JobError::InvalidJobSpec {
                reason: "Not a blob reference".to_string(),
            });
        }

        let hash_str = value.get("hash").and_then(|v| v.as_str()).ok_or_else(|| JobError::InvalidJobSpec {
            reason: "Missing blob hash".to_string(),
        })?;

        let hash = BlobHash::from_hex(hash_str)?;

        let format = value
            .get("format")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(PayloadFormat::Json);

        let size = value.get("size").and_then(|v| v.as_u64()).unwrap_or(0);

        let metadata = value.get("metadata").cloned().unwrap_or_else(|| serde_json::json!({}));

        Ok(BlobPayload {
            blob_hash: hash,
            format,
            size,
            metadata,
        })
    }

    /// Get blob store statistics.
    pub async fn get_stats(&self) -> BlobStats {
        let blobs = self.store.blobs.read().await;
        let total_size: usize = blobs.values().map(|b| b.len()).sum();

        BlobStats {
            total_blobs: blobs.len(),
            total_size_bytes: total_size,
            average_size_bytes: if blobs.is_empty() { 0 } else { total_size / blobs.len() },
        }
    }
}

/// Blob storage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobStats {
    /// Total number of blobs.
    pub total_blobs: usize,
    /// Total size in bytes.
    pub total_size_bytes: usize,
    /// Average blob size.
    pub average_size_bytes: usize,
}

/// Extension for JobManager with blob storage support.
pub struct BlobJobManager<S: aspen_core::KeyValueStore + ?Sized> {
    manager: Arc<JobManager<S>>,
    blob_storage: Arc<JobBlobStorage>,
}

impl<S: aspen_core::KeyValueStore + ?Sized + 'static> BlobJobManager<S> {
    /// Create a new blob-enabled job manager.
    pub async fn new(manager: Arc<JobManager<S>>) -> Result<Self> {
        let blob_storage = Arc::new(JobBlobStorage::new().await?);

        Ok(Self { manager, blob_storage })
    }

    /// Submit a job with automatic blob storage for large payloads.
    pub async fn submit_with_blob(&self, mut spec: JobSpec) -> Result<JobId> {
        // Check if payload should be stored as blob
        if JobBlobStorage::should_use_blob(&spec.payload) {
            // Generate a temporary ID for blob storage
            let temp_id = uuid::Uuid::new_v4().to_string();

            // Store payload in blob
            let blob_payload = self.blob_storage.store_payload(&spec.payload, &temp_id).await?;

            // Replace payload with blob reference
            spec.payload = JobBlobStorage::create_blob_reference(&blob_payload);

            info!(
                blob_hash = %blob_payload.blob_hash.to_hex(),
                "stored large payload in blob storage"
            );
        }

        self.manager.submit(spec).await
    }

    /// Retrieve a job with automatic blob fetching.
    pub async fn get_job_with_blob(&self, job_id: &JobId) -> Result<Option<crate::job::Job>> {
        if let Some(mut job) = self.manager.get_job(job_id).await? {
            // Check if payload is a blob reference
            if JobBlobStorage::is_blob_reference(&job.spec.payload) {
                let blob_payload = JobBlobStorage::extract_blob_payload(&job.spec.payload)?;

                // Fetch actual payload from blob storage
                job.spec.payload = self.blob_storage.retrieve_payload(&blob_payload).await?;

                info!(
                    job_id = %job_id,
                    "retrieved payload from blob storage"
                );
            }

            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    /// Store large job result.
    pub async fn store_job_result(&self, job_id: &str, result: serde_json::Value) -> Result<()> {
        // Check if result should be stored as blob
        if JobBlobStorage::should_use_blob(&result) {
            let blob_payload = self.blob_storage.store_payload(&result, job_id).await?;

            // Create blob reference for future use (in real implementation, this would update the job)
            let _blob_reference = JobBlobStorage::create_blob_reference(&blob_payload);

            info!(
                job_id = %job_id,
                blob_hash = %blob_payload.blob_hash.to_hex(),
                "stored large result in blob storage"
            );
        }

        // In real implementation, this would update the job status
        Ok(())
    }

    /// Get blob storage statistics.
    pub async fn get_blob_stats(&self) -> BlobStats {
        self.blob_storage.get_stats().await
    }
}

/// Blob collection for batch operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobCollection {
    /// Collection ID.
    pub id: String,
    /// Blobs in the collection.
    pub blobs: Vec<BlobHash>,
    /// Total size in bytes.
    pub total_size: u64,
    /// Collection metadata.
    pub metadata: serde_json::Value,
}

impl BlobCollection {
    /// Create a new blob collection.
    pub fn new(id: String) -> Self {
        Self {
            id,
            blobs: Vec::new(),
            total_size: 0,
            metadata: serde_json::json!({}),
        }
    }

    /// Add a blob to the collection.
    pub fn add_blob(&mut self, hash: BlobHash, size: u64) {
        self.blobs.push(hash);
        self.total_size += size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_threshold() {
        let small_payload = serde_json::json!({ "data": "small" });
        assert!(!JobBlobStorage::should_use_blob(&small_payload));

        let large_string = "x".repeat(2 * 1024 * 1024);
        let large_payload = serde_json::json!({ "data": large_string });
        assert!(JobBlobStorage::should_use_blob(&large_payload));
    }

    #[test]
    fn test_blob_reference() {
        let hash = BlobHash::from_hex("0123456789abcdef").unwrap();
        let blob_payload = BlobPayload {
            blob_hash: hash.clone(),
            format: PayloadFormat::Json,
            size: 1000,
            metadata: serde_json::json!({}),
        };

        let reference = JobBlobStorage::create_blob_reference(&blob_payload);
        assert!(JobBlobStorage::is_blob_reference(&reference));

        let extracted = JobBlobStorage::extract_blob_payload(&reference).unwrap();
        assert_eq!(extracted.blob_hash, hash);
        assert_eq!(extracted.size, 1000);
    }

    #[tokio::test]
    async fn test_compression() {
        let storage = JobBlobStorage::new().await.unwrap();

        let data = b"Hello World! ".repeat(100);
        let compressed = storage.compress_data(&data).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = storage.decompress_data(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[tokio::test]
    async fn test_bounded_decompression_within_limit() {
        let storage = JobBlobStorage::new().await.unwrap();

        // Create data that's well within the limit
        let data = vec![0u8; 1024 * 1024]; // 1 MB of zeros (compresses very well)
        let compressed = storage.compress_data(&data).unwrap();

        // Decompression should succeed
        let decompressed = storage.decompress_data(&compressed).unwrap();
        assert_eq!(decompressed.len(), data.len());
    }

    #[tokio::test]
    async fn test_compression_bomb_rejected() {
        use std::io::Write;

        use flate2::Compression;
        use flate2::write::GzEncoder;

        let storage = JobBlobStorage::new().await.unwrap();

        // Create a "compression bomb" - highly compressible data that exceeds limit
        // 200 MB of zeros compresses to very small size but would expand beyond MAX_DECOMPRESSED_SIZE
        let bomb_size = super::MAX_DECOMPRESSED_SIZE + 1024 * 1024; // 101 MB
        let bomb_data = vec![0u8; bomb_size];

        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&bomb_data).unwrap();
        let compressed_bomb = encoder.finish().unwrap();

        // Verify the bomb compresses well (this is what makes it dangerous)
        assert!(compressed_bomb.len() < 1024 * 1024, "Bomb should compress to < 1MB");

        // Decompression should fail with DecompressionTooLarge error
        let result = storage.decompress_data(&compressed_bomb);
        assert!(result.is_err(), "Compression bomb should be rejected");

        match result.unwrap_err() {
            crate::error::JobError::DecompressionTooLarge { max } => {
                assert_eq!(max, super::MAX_DECOMPRESSED_SIZE);
            }
            other => panic!("Expected DecompressionTooLarge error, got: {:?}", other),
        }
    }

    #[test]
    fn test_blob_hash() {
        let data1 = b"test data";
        let data2 = b"different data";

        let hash1 = BlobHash::from_data(data1);
        let hash2 = BlobHash::from_data(data2);

        assert_ne!(hash1, hash2);
        assert_eq!(hash1, BlobHash::from_data(data1));
    }
}
