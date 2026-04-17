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

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "blob worker measures operation latency with a monotonic clock helper"
)]
fn current_instant() -> Instant {
    Instant::now()
}

fn elapsed_ms_u64(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

impl BlobProcessorWorker {
    /// Create a new blob processor worker with blob store access.
    pub fn new(node_id: u64, blob_store: Arc<dyn BlobStore>) -> Self {
        Self { node_id, blob_store }
    }

    /// Validate a blob by checking existence and verifying content hash.
    async fn validate_blob(&self, hash_str: &str) -> Result<serde_json::Value, String> {
        let start = current_instant();

        // Parse the hash
        let hash = self.parse_hash(hash_str)?;

        // Check if blob exists
        let is_present =
            self.blob_store.has(&hash).await.map_err(|e| format!("failed to check blob existence: {}", e))?;

        if !is_present {
            return Ok(json!({
                "node_id": self.node_id,
                "blob_hash": hash_str,
                "valid": false,
                "exists": false,
                "error": "blob not found",
                "validation_time_ms": elapsed_ms_u64(start)
            }));
        }

        // Get blob bytes and verify hash
        let bytes = self
            .blob_store
            .get_bytes(&hash)
            .await
            .map_err(|e| format!("failed to get blob: {}", e))?
            .ok_or("blob exists but could not be retrieved")?;

        let size_bytes = bytes.len();

        // Verify content hash matches (iroh-blobs uses BLAKE3)
        let computed_hash = Hash::new(&bytes);
        let is_valid = computed_hash == hash;

        let validation_time_ms = elapsed_ms_u64(start);

        Ok(json!({
            "node_id": self.node_id,
            "blob_hash": hash_str,
            "valid": is_valid,
            "exists": true,
            "size_bytes": size_bytes,
            "hash_verified": is_valid,
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
        let start = current_instant();

        let hash = self.parse_hash(hash_str)?;

        // Check if blob exists locally
        let is_present = self.blob_store.has(&hash).await.map_err(|e| format!("failed to check blob: {}", e))?;

        if !is_present {
            return Ok(json!({
                "node_id": self.node_id,
                "blob_hash": hash_str,
                "success": false,
                "error": "blob not found locally",
                "replication_time_ms": elapsed_ms_u64(start)
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

        let replication_time_ms = elapsed_ms_u64(start);

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
        let start = current_instant();

        let hash = self.parse_hash(hash_str)?;

        // Get the original blob
        let original_bytes = self
            .blob_store
            .get_bytes(&hash)
            .await
            .map_err(|e| format!("failed to get blob: {}", e))?
            .ok_or("blob not found")?;

        let original_size_bytes = original_bytes.len();

        // Compress based on algorithm
        let compressed = match algorithm.to_lowercase().as_str() {
            "gzip" | "gz" => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::new(6));
                encoder.write_all(&original_bytes).map_err(|e| format!("gzip compression failed: {}", e))?;
                encoder.finish().map_err(|e| format!("gzip finalization failed: {}", e))?
            }
            "zstd" => zstd_compress(&original_bytes).map_err(|e| format!("zstd compression failed: {}", e))?,
            _ => {
                return Err(format!("unsupported compression algorithm '{}', use 'gzip' or 'zstd'", algorithm));
            }
        };

        let compressed_size_bytes = compressed.len();

        // Store the compressed blob
        let result = self
            .blob_store
            .add_bytes(&compressed)
            .await
            .map_err(|e| format!("failed to store compressed blob: {}", e))?;

        let compressed_hash = result.blob_ref.hash.to_hex().to_string();
        let compression_ratio = if original_size_bytes > 0 {
            compressed_size_bytes as f64 / original_size_bytes as f64
        } else {
            1.0
        };

        let compression_time_ms = elapsed_ms_u64(start);

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
            "original_size": original_size_bytes,
            "compressed_size": compressed_size_bytes,
            "compression_ratio": compression_ratio,
            "space_saved_bytes": original_size_bytes as i64 - compressed_size_bytes as i64,
            "compression_time_ms": compression_time_ms
        }))
    }

    /// Extract metadata from a blob.
    ///
    /// Attempts to detect content type and extract relevant metadata.
    async fn extract_metadata(&self, hash_str: &str) -> Result<serde_json::Value, String> {
        let start = current_instant();

        let hash = self.parse_hash(hash_str)?;

        // Get blob bytes
        let bytes = self
            .blob_store
            .get_bytes(&hash)
            .await
            .map_err(|e| format!("failed to get blob: {}", e))?
            .ok_or("blob not found")?;

        let size_bytes = bytes.len();

        // Detect content type from magic bytes
        let content_type = detect_content_type(&bytes);

        // Extract additional metadata based on content type
        let mut metadata = serde_json::Map::new();
        metadata.insert("content_type".to_string(), json!(content_type));
        metadata.insert("size_bytes".to_string(), json!(size_bytes));

        // Check if it's text
        if content_type.starts_with("text/") || content_type == "application/json" {
            if let Ok(text) = std::str::from_utf8(&bytes) {
                metadata.insert("is_utf8".to_string(), json!(true));
                metadata.insert("line_count".to_string(), json!(text.lines().count()));
                metadata.insert("char_count".to_string(), json!(text.chars().count()));

                // For JSON, try to parse and get structure info
                if content_type == "application/json"
                    && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text)
                {
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
            } else {
                metadata.insert("is_utf8".to_string(), json!(false));
            }
        }

        // For images, extract basic info
        if content_type.starts_with("image/")
            && let Some((width, height)) = extract_image_dimensions(&bytes)
        {
            metadata.insert("width".to_string(), json!(width));
            metadata.insert("height".to_string(), json!(height));
        }

        let extraction_time_ms = elapsed_ms_u64(start);

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

fn detect_magic_content_type(bytes: &[u8]) -> Option<&'static str> {
    match &bytes[..4] {
        [0x89, b'P', b'N', b'G'] => Some("image/png"),
        [0xFF, 0xD8, 0xFF, _] => Some("image/jpeg"),
        [b'G', b'I', b'F', b'8'] => Some("image/gif"),
        [b'R', b'I', b'F', b'F'] if bytes.len() >= 12 && &bytes[8..12] == b"WEBP" => Some("image/webp"),
        [0x50, 0x4B, 0x03, 0x04] => Some("application/zip"),
        [0x1F, 0x8B, _, _] => Some("application/gzip"),
        [0x28, 0xB5, 0x2F, 0xFD] => Some("application/zstd"),
        [b'%', b'P', b'D', b'F'] => Some("application/pdf"),
        [b'{', _, _, _] | [b'[', _, _, _] => Some("application/json"),
        [b'<', b'?', b'x', b'm'] => Some("application/xml"),
        [b'<', b'!', b'D', b'O'] | [b'<', b'h', b't', b'm'] | [b'<', b'H', b'T', b'M'] => Some("text/html"),
        [0x7F, b'E', b'L', b'F'] => Some("application/x-executable"),
        [0xCA, 0xFE, 0xBA, 0xBE] | [0xCF, 0xFA, 0xED, 0xFE] => Some("application/x-mach-binary"),
        [b'M', b'Z', _, _] => Some("application/x-dosexec"),
        [0x00, b'a', b's', b'm'] => Some("application/wasm"),
        [b'S', b'Q', b'L', b'i'] => Some("application/x-sqlite3"),
        _ => None,
    }
}

fn looks_like_utf8_text_prefix(bytes: &[u8]) -> bool {
    bytes.iter().take(1024).all(|&byte| {
        byte.is_ascii_alphanumeric() || byte.is_ascii_punctuation() || byte.is_ascii_whitespace() || byte >= 0x80
    })
}

/// Detect content type from magic bytes.
fn detect_content_type(bytes: &[u8]) -> String {
    if bytes.len() < 4 {
        return "application/octet-stream".to_string();
    }
    if let Some(content_type) = detect_magic_content_type(bytes) {
        return content_type.to_string();
    }
    if looks_like_utf8_text_prefix(bytes) {
        "text/plain".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

fn read_u16_be(bytes: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(2)?;
    let slice = bytes.get(offset..end)?;
    let pair: [u8; 2] = slice.try_into().ok()?;
    Some(u16::from_be_bytes(pair))
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let slice = bytes.get(offset..end)?;
    let quad: [u8; 4] = slice.try_into().ok()?;
    Some(u32::from_be_bytes(quad))
}

fn is_jpeg_sof_marker(marker: u8) -> bool {
    (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC)
}

fn extract_png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    if !bytes.starts_with(&PNG_MAGIC) {
        return None;
    }
    let width = read_u32_be(bytes, 16)?;
    let height = read_u32_be(bytes, 20)?;
    Some((width, height))
}

fn parse_jpeg_segment(bytes: &[u8], offset_bytes: usize) -> Option<(u8, usize)> {
    if *bytes.get(offset_bytes)? != 0xFF {
        return None;
    }
    let marker = *bytes.get(offset_bytes.checked_add(1)?)?;
    let segment_len_bytes = usize::from(read_u16_be(bytes, offset_bytes.checked_add(2)?)?);
    Some((marker, segment_len_bytes))
}

fn jpeg_sof_dimensions(bytes: &[u8], offset_bytes: usize) -> Option<(u32, u32)> {
    let height = u32::from(read_u16_be(bytes, offset_bytes.checked_add(5)?)?);
    let width = u32::from(read_u16_be(bytes, offset_bytes.checked_add(7)?)?);
    Some((width, height))
}

fn extract_jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return None;
    }
    let mut offset_bytes = 2usize;
    while let Some(min_segment_end) = offset_bytes.checked_add(8) {
        debug_assert!(offset_bytes >= 2, "jpeg scan stays after SOI bytes");
        debug_assert!(min_segment_end >= offset_bytes, "checked_add keeps scan bound monotonic");
        if min_segment_end >= bytes.len() {
            return None;
        }
        if let Some((marker, segment_len_bytes)) = parse_jpeg_segment(bytes, offset_bytes) {
            debug_assert!(segment_len_bytes >= 2, "jpeg segment length includes its length field");
            if is_jpeg_sof_marker(marker) {
                return jpeg_sof_dimensions(bytes, offset_bytes);
            }
            offset_bytes = offset_bytes.checked_add(2)?.checked_add(segment_len_bytes)?;
            continue;
        }
        offset_bytes = offset_bytes.checked_add(1)?;
    }
    None
}

/// Extract image dimensions from PNG/JPEG headers.
fn extract_image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 {
        return None;
    }
    extract_png_dimensions(bytes).or_else(|| extract_jpeg_dimensions(bytes))
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
