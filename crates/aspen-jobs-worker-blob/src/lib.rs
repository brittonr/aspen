//! Blob processing worker for content-addressed data operations.
//!
//! This worker handles blob operations including validation, replication,
//! compression, and metadata extraction. It uses iroh-blobs for content-addressed
//! storage with BLAKE3 hashing.
//!
//! Supported job types:
//! - `validate_blob`: Verify blob integrity and existence
//! - `replicate_blob`: Replicate blob to other nodes
//! - `compress_blob`: Compress blob using zstd/gzip
//! - `extract_metadata`: Extract metadata from blob content

use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use aspen_blob::prelude::*;
use aspen_jobs::Job;
use aspen_jobs::JobResult;
use aspen_jobs::Worker;
use async_trait::async_trait;
use flate2::Compression;
use flate2::write::GzEncoder;
use iroh_blobs::Hash;
use serde_json::json;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Worker for processing iroh-blobs content.
///
/// This worker requires access to a blob store for all operations.
/// It can validate, replicate, compress, and extract metadata from blobs.
pub struct BlobProcessorWorker {
    node_id: u64,
    /// Blob store for all operations.
    blob_store: Arc<dyn BlobStore>,
}

impl BlobProcessorWorker {
    /// Create a new blob processor worker with blob store access.
    pub fn new(node_id: u64, blob_store: Arc<dyn BlobStore>) -> Self {
        Self { node_id, blob_store }
    }

    /// Validate a blob by checking existence and verifying content hash.
    async fn validate_blob(&self, hash_str: &str) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        // Parse the hash
        let hash = self.parse_hash(hash_str)?;

        // Check if blob exists
        let exists = self.blob_store.has(&hash).await.map_err(|e| format!("failed to check blob existence: {}", e))?;

        if !exists {
            return Ok(json!({
                "node_id": self.node_id,
                "blob_hash": hash_str,
                "valid": false,
                "exists": false,
                "error": "blob not found",
                "validation_time_ms": start.elapsed().as_millis() as u64
            }));
        }

        // Get blob bytes and verify hash
        let bytes = self
            .blob_store
            .get_bytes(&hash)
            .await
            .map_err(|e| format!("failed to get blob: {}", e))?
            .ok_or("blob exists but could not be retrieved")?;

        let size = bytes.len();

        // Verify content hash matches (iroh-blobs uses BLAKE3)
        let computed_hash = Hash::new(&bytes);
        let valid = computed_hash == hash;

        let validation_time_ms = start.elapsed().as_millis() as u64;

        Ok(json!({
            "node_id": self.node_id,
            "blob_hash": hash_str,
            "valid": valid,
            "exists": true,
            "size_bytes": size,
            "hash_verified": valid,
            "validation_time_ms": validation_time_ms
        }))
    }

    /// Replicate a blob to target nodes.
    ///
    /// This generates a ticket for the blob that can be shared with other nodes.
    async fn replicate_blob(
        &self,
        hash_str: &str,
        target_nodes: &[serde_json::Value],
    ) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let hash = self.parse_hash(hash_str)?;

        // Check if blob exists locally
        let exists = self.blob_store.has(&hash).await.map_err(|e| format!("failed to check blob: {}", e))?;

        if !exists {
            return Ok(json!({
                "node_id": self.node_id,
                "blob_hash": hash_str,
                "success": false,
                "error": "blob not found locally",
                "replication_time_ms": start.elapsed().as_millis() as u64
            }));
        }

        // Generate a ticket for sharing
        let ticket = self.blob_store.ticket(&hash).await.map_err(|e| format!("failed to create ticket: {}", e))?;

        // Protect blob from GC during replication
        let tag_name = format!("replication:{}", hash_str);
        self.blob_store
            .protect(&hash, &tag_name)
            .await
            .map_err(|e| format!("failed to protect blob: {}", e))?;

        let replication_time_ms = start.elapsed().as_millis() as u64;

        info!(
            node_id = self.node_id,
            hash = %hash_str,
            target_count = target_nodes.len(),
            "generated replication ticket for blob"
        );

        Ok(json!({
            "node_id": self.node_id,
            "blob_hash": hash_str,
            "success": true,
            "ticket": ticket.to_string(),
            "target_nodes": target_nodes.len(),
            "protection_tag": tag_name,
            "replication_time_ms": replication_time_ms
        }))
    }

    /// Compress a blob using the specified algorithm.
    async fn compress_blob(&self, hash_str: &str, algorithm: &str) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let hash = self.parse_hash(hash_str)?;

        // Get the original blob
        let original_bytes = self
            .blob_store
            .get_bytes(&hash)
            .await
            .map_err(|e| format!("failed to get blob: {}", e))?
            .ok_or("blob not found")?;

        let original_size = original_bytes.len();

        // Compress based on algorithm
        let compressed = match algorithm.to_lowercase().as_str() {
            "gzip" | "gz" => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&original_bytes).map_err(|e| format!("gzip compression failed: {}", e))?;
                encoder.finish().map_err(|e| format!("gzip finalization failed: {}", e))?
            }
            "zstd" => zstd_compress(&original_bytes).map_err(|e| format!("zstd compression failed: {}", e))?,
            _ => {
                return Err(format!("unsupported compression algorithm '{}', use 'gzip' or 'zstd'", algorithm));
            }
        };

        let compressed_size = compressed.len();

        // Store the compressed blob
        let result = self
            .blob_store
            .add_bytes(&compressed)
            .await
            .map_err(|e| format!("failed to store compressed blob: {}", e))?;

        let compressed_hash = result.blob_ref.hash.to_hex().to_string();
        let compression_ratio = if original_size > 0 {
            compressed_size as f64 / original_size as f64
        } else {
            1.0
        };

        let compression_time_ms = start.elapsed().as_millis() as u64;

        info!(
            node_id = self.node_id,
            original_hash = %hash_str,
            compressed_hash = %compressed_hash,
            algorithm = algorithm,
            ratio = compression_ratio,
            "blob compressed"
        );

        Ok(json!({
            "node_id": self.node_id,
            "original_hash": hash_str,
            "compressed_hash": compressed_hash,
            "algorithm": algorithm,
            "original_size": original_size,
            "compressed_size": compressed_size,
            "compression_ratio": compression_ratio,
            "space_saved_bytes": original_size as i64 - compressed_size as i64,
            "compression_time_ms": compression_time_ms
        }))
    }

    /// Extract metadata from a blob.
    ///
    /// Attempts to detect content type and extract relevant metadata.
    async fn extract_metadata(&self, hash_str: &str) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let hash = self.parse_hash(hash_str)?;

        // Get blob bytes
        let bytes = self
            .blob_store
            .get_bytes(&hash)
            .await
            .map_err(|e| format!("failed to get blob: {}", e))?
            .ok_or("blob not found")?;

        let size = bytes.len();

        // Detect content type from magic bytes
        let content_type = detect_content_type(&bytes);

        // Extract additional metadata based on content type
        let mut metadata = serde_json::Map::new();
        metadata.insert("content_type".to_string(), json!(content_type));
        metadata.insert("size_bytes".to_string(), json!(size));

        // Check if it's text
        if content_type.starts_with("text/") || content_type == "application/json" {
            if let Ok(text) = std::str::from_utf8(&bytes) {
                metadata.insert("is_utf8".to_string(), json!(true));
                metadata.insert("line_count".to_string(), json!(text.lines().count()));
                metadata.insert("char_count".to_string(), json!(text.chars().count()));

                // For JSON, try to parse and get structure info
                if content_type == "application/json" {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                        metadata.insert("json_valid".to_string(), json!(true));
                        match &parsed {
                            serde_json::Value::Object(obj) => {
                                metadata.insert("json_type".to_string(), json!("object"));
                                metadata.insert("json_keys".to_string(), json!(obj.len()));
                            }
                            serde_json::Value::Array(arr) => {
                                metadata.insert("json_type".to_string(), json!("array"));
                                metadata.insert("json_length".to_string(), json!(arr.len()));
                            }
                            _ => {
                                metadata.insert("json_type".to_string(), json!("primitive"));
                            }
                        }
                    }
                }
            } else {
                metadata.insert("is_utf8".to_string(), json!(false));
            }
        }

        // For images, extract basic info
        if content_type.starts_with("image/") {
            if let Some((width, height)) = extract_image_dimensions(&bytes) {
                metadata.insert("width".to_string(), json!(width));
                metadata.insert("height".to_string(), json!(height));
            }
        }

        let extraction_time_ms = start.elapsed().as_millis() as u64;

        debug!(
            node_id = self.node_id,
            hash = %hash_str,
            content_type = content_type,
            "metadata extracted"
        );

        Ok(json!({
            "node_id": self.node_id,
            "blob_hash": hash_str,
            "metadata": metadata,
            "extraction_time_ms": extraction_time_ms
        }))
    }

    /// Parse a hash string to a Hash.
    fn parse_hash(&self, hash_str: &str) -> Result<Hash, String> {
        // Handle both hex and base32 formats
        if hash_str.len() == 64 {
            // Hex format
            let mut bytes = [0u8; 32];
            hex::decode_to_slice(hash_str, &mut bytes).map_err(|e| format!("invalid hex hash: {}", e))?;
            Ok(Hash::from(bytes))
        } else {
            // Try base32 (iroh default format)
            hash_str.parse::<Hash>().map_err(|e| format!("invalid hash format: {}", e))
        }
    }
}

/// Compress data using zstd.
fn zstd_compress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    // Use default compression level (3)
    let mut encoder = zstd::stream::Encoder::new(Vec::new(), 3)?;
    encoder.write_all(data)?;
    encoder.finish()
}

/// Detect content type from magic bytes.
fn detect_content_type(bytes: &[u8]) -> String {
    if bytes.len() < 4 {
        return "application/octet-stream".to_string();
    }

    // Check magic bytes
    match &bytes[..4] {
        // Images
        [0x89, b'P', b'N', b'G'] => "image/png".to_string(),
        [0xFF, 0xD8, 0xFF, _] => "image/jpeg".to_string(),
        [b'G', b'I', b'F', b'8'] => "image/gif".to_string(),
        [b'R', b'I', b'F', b'F'] if bytes.len() >= 12 && &bytes[8..12] == b"WEBP" => "image/webp".to_string(),
        // Archives
        [0x50, 0x4B, 0x03, 0x04] => "application/zip".to_string(),
        [0x1F, 0x8B, _, _] => "application/gzip".to_string(),
        [0x28, 0xB5, 0x2F, 0xFD] => "application/zstd".to_string(),
        // Documents
        [b'%', b'P', b'D', b'F'] => "application/pdf".to_string(),
        // Text/Data
        [b'{', _, _, _] | [b'[', _, _, _] => "application/json".to_string(),
        [b'<', b'?', b'x', b'm'] => "application/xml".to_string(),
        [b'<', b'!', b'D', b'O'] | [b'<', b'h', b't', b'm'] | [b'<', b'H', b'T', b'M'] => "text/html".to_string(),
        // Binary formats
        [0x7F, b'E', b'L', b'F'] => "application/x-executable".to_string(),
        [0xCA, 0xFE, 0xBA, 0xBE] | [0xCF, 0xFA, 0xED, 0xFE] => "application/x-mach-binary".to_string(),
        [b'M', b'Z', _, _] => "application/x-dosexec".to_string(),
        // Wasm
        [0x00, b'a', b's', b'm'] => "application/wasm".to_string(),
        // SQLite
        [b'S', b'Q', b'L', b'i'] => "application/x-sqlite3".to_string(),
        // Try to detect text
        _ => {
            // Check if it looks like UTF-8 text
            if bytes.iter().take(1024).all(|&b| {
                b.is_ascii_alphanumeric() || b.is_ascii_punctuation() || b.is_ascii_whitespace() || b >= 0x80 // Could be valid UTF-8 continuation
            }) {
                "text/plain".to_string()
            } else {
                "application/octet-stream".to_string()
            }
        }
    }
}

/// Extract image dimensions from PNG/JPEG headers.
fn extract_image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 {
        return None;
    }

    // PNG: dimensions at bytes 16-23 (width at 16, height at 20)
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) && bytes.len() >= 24 {
        let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return Some((width, height));
    }

    // JPEG: Look for SOF0 marker (0xFF 0xC0)
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        let mut i = 2;
        while i < bytes.len() - 8 {
            if bytes[i] == 0xFF {
                let marker = bytes[i + 1];
                // SOF markers: 0xC0-0xCF (except 0xC4, 0xC8, 0xCC)
                if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
                    let height = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                    let width = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                    return Some((width, height));
                }
                // Skip this segment
                if i + 3 < bytes.len() {
                    let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
                    i += 2 + len;
                } else {
                    break;
                }
            } else {
                i += 1;
            }
        }
    }

    None
}

#[async_trait]
impl Worker for BlobProcessorWorker {
    async fn execute(&self, job: Job) -> JobResult {
        match job.spec.job_type.as_str() {
            "validate_blob" => {
                let blob_hash = job.spec.payload["hash"].as_str().unwrap_or("unknown");

                info!(node_id = self.node_id, blob_hash = blob_hash, "validating blob integrity");

                match self.validate_blob(blob_hash).await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "blob validation failed");
                        JobResult::failure(format!("validation failed: {}", e))
                    }
                }
            }

            "replicate_blob" => {
                let blob_hash = job.spec.payload["hash"].as_str().unwrap_or("unknown");
                let target_nodes =
                    job.spec.payload["target_nodes"].as_array().map(|arr| arr.to_vec()).unwrap_or_default();

                info!(
                    node_id = self.node_id,
                    blob_hash = blob_hash,
                    target_nodes = target_nodes.len(),
                    "replicating blob to target nodes"
                );

                match self.replicate_blob(blob_hash, &target_nodes).await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "blob replication failed");
                        JobResult::failure(format!("replication failed: {}", e))
                    }
                }
            }

            "compress_blob" => {
                let blob_hash = job.spec.payload["hash"].as_str().unwrap_or("unknown");
                let algorithm = job.spec.payload["algorithm"].as_str().unwrap_or("zstd");

                info!(node_id = self.node_id, blob_hash = blob_hash, algorithm = algorithm, "compressing blob");

                match self.compress_blob(blob_hash, algorithm).await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "blob compression failed");
                        JobResult::failure(format!("compression failed: {}", e))
                    }
                }
            }

            "extract_metadata" => {
                let blob_hash = job.spec.payload["hash"].as_str().unwrap_or("unknown");

                info!(node_id = self.node_id, blob_hash = blob_hash, "extracting blob metadata");

                match self.extract_metadata(blob_hash).await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "metadata extraction failed");
                        JobResult::failure(format!("metadata extraction failed: {}", e))
                    }
                }
            }

            _ => JobResult::failure(format!("unknown blob processing task: {}", job.spec.job_type)),
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec![
            "validate_blob".to_string(),
            "replicate_blob".to_string(),
            "compress_blob".to_string(),
            "extract_metadata".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_content_type() {
        // PNG
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_content_type(&png), "image/png");

        // JPEG
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_content_type(&jpeg), "image/jpeg");

        // JSON
        let json = b"{\"hello\": \"world\"}";
        assert_eq!(detect_content_type(json), "application/json");

        // GZIP
        let gzip = [0x1F, 0x8B, 0x08, 0x00];
        assert_eq!(detect_content_type(&gzip), "application/gzip");

        // Plain text
        let text = b"Hello, world!";
        assert_eq!(detect_content_type(text), "text/plain");
    }

    #[test]
    fn test_extract_png_dimensions() {
        // Minimal PNG header with 100x200 dimensions
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]); // IHDR length
        png.extend_from_slice(b"IHDR"); // IHDR type
        png.extend_from_slice(&100u32.to_be_bytes()); // Width
        png.extend_from_slice(&200u32.to_be_bytes()); // Height

        let dims = extract_image_dimensions(&png);
        assert_eq!(dims, Some((100, 200)));
    }
}
