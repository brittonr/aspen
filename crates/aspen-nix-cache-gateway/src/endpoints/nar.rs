//! Handler for GET /nar/{hash}.nar endpoint.

use aspen_blob::BlobStore;
use aspen_cache::CacheIndex;
use http::{Response, StatusCode};
use iroh_blobs::Hash;
use tracing::{debug, instrument};

use crate::error::{NixCacheError, Result};

/// HTTP Range specification.
#[derive(Debug, Clone, Copy)]
pub struct HttpRange {
    /// Start byte offset (inclusive).
    pub start: u64,
    /// End byte offset (inclusive), or None for "to end of file".
    pub end: Option<u64>,
}

impl HttpRange {
    /// Parse a Range header value.
    ///
    /// Supports format: `bytes=start-end` or `bytes=start-`
    pub fn parse(header: &str) -> Result<Self> {
        let header = header.trim();
        let bytes_range = header.strip_prefix("bytes=").ok_or_else(|| NixCacheError::InvalidRange {
            reason: "missing 'bytes=' prefix".to_string(),
        })?;

        let parts: Vec<&str> = bytes_range.splitn(2, '-').collect();
        if parts.len() != 2 {
            return Err(NixCacheError::InvalidRange {
                reason: "invalid range format".to_string(),
            });
        }

        let start: u64 = parts[0].parse().map_err(|_| NixCacheError::InvalidRange {
            reason: "invalid start offset".to_string(),
        })?;

        let end = if parts[1].is_empty() {
            None
        } else {
            Some(parts[1].parse().map_err(|_| NixCacheError::InvalidRange {
                reason: "invalid end offset".to_string(),
            })?)
        };

        Ok(HttpRange { start, end })
    }

    /// Resolve the range to actual byte offsets given content length.
    ///
    /// Returns (start, end) inclusive byte offsets.
    pub fn resolve(&self, content_length: u64) -> Result<(u64, u64)> {
        if content_length == 0 {
            return Err(NixCacheError::RangeNotSatisfiable {
                requested: format!("{}-{:?}", self.start, self.end),
                blob_size: 0,
            });
        }

        let end = self.end.unwrap_or(content_length - 1);

        if self.start >= content_length || end >= content_length || self.start > end {
            return Err(NixCacheError::RangeNotSatisfiable {
                requested: format!("{}-{}", self.start, end),
                blob_size: content_length,
            });
        }

        Ok((self.start, end))
    }
}

/// NAR download metadata.
///
/// Used to prepare a streaming response.
pub struct NarDownload {
    /// Blob hash to download.
    pub blob_hash: Hash,
    /// Total content length.
    pub content_length: u64,
    /// NAR hash for headers.
    pub nar_hash: String,
    /// Byte range (start, end inclusive).
    pub range: Option<(u64, u64)>,
}

impl NarDownload {
    /// Check if this is a partial (range) request.
    pub fn is_partial(&self) -> bool {
        self.range.is_some()
    }

    /// Get the transfer size.
    pub fn transfer_size(&self) -> u64 {
        match self.range {
            Some((start, end)) => end - start + 1,
            None => self.content_length,
        }
    }

    /// Build HTTP response headers (without body).
    pub fn response_headers(&self) -> Response<()> {
        let mut builder = Response::builder()
            .header("Content-Type", "application/x-nix-nar")
            .header("Accept-Ranges", "bytes")
            .header("X-Nar-Hash", &self.nar_hash);

        if let Some((start, end)) = self.range {
            builder = builder
                .status(StatusCode::PARTIAL_CONTENT)
                .header("Content-Length", end - start + 1)
                .header(
                    "Content-Range",
                    format!("bytes {}-{}/{}", start, end, self.content_length),
                );
        } else {
            builder = builder
                .status(StatusCode::OK)
                .header("Content-Length", self.content_length);
        }

        builder.body(()).expect("valid response")
    }
}

/// Prepare a NAR download.
///
/// Looks up the cache entry, validates the blob exists, and parses any range header.
#[instrument(skip(cache_index, blob_store), fields(blob_hash = %blob_hash))]
pub async fn prepare_nar_download<I, B>(
    blob_hash: &str,
    range_header: Option<&str>,
    cache_index: &I,
    blob_store: &B,
) -> Result<NarDownload>
where
    I: CacheIndex,
    B: BlobStore,
{
    // Parse blob hash
    let hash: Hash = blob_hash.parse().map_err(|_| NixCacheError::InvalidStoreHash {
        hash: blob_hash.to_string(),
    })?;

    // Check blob exists
    if !blob_store.has(&hash).await.map_err(|e| NixCacheError::BlobStore {
        message: e.to_string(),
    })? {
        return Err(NixCacheError::BlobNotAvailable {
            blob_hash: blob_hash.to_string(),
        });
    }

    // Get blob status for size
    let status = blob_store.status(&hash).await.map_err(|e| NixCacheError::BlobStore {
        message: e.to_string(),
    })?;

    let content_length = status
        .and_then(|s| s.size)
        .ok_or_else(|| NixCacheError::BlobStore {
            message: "blob size unknown".to_string(),
        })?;

    // Parse range header if present
    let range = if let Some(header) = range_header {
        let http_range = HttpRange::parse(header)?;
        Some(http_range.resolve(content_length)?)
    } else {
        None
    };

    // Try to find NAR hash from cache (for X-Nar-Hash header)
    // This is optional - we might not have the entry if downloading by blob hash directly
    let nar_hash = find_nar_hash_for_blob(blob_hash, cache_index).await.unwrap_or_default();

    debug!(
        content_length,
        range = ?range,
        "NAR download prepared"
    );

    Ok(NarDownload {
        blob_hash: hash,
        content_length,
        nar_hash,
        range,
    })
}

/// Find the NAR hash for a blob hash by scanning the cache.
///
/// This is a best-effort lookup - returns empty string if not found.
async fn find_nar_hash_for_blob<I>(_blob_hash: &str, _cache_index: &I) -> Option<String>
where
    I: CacheIndex,
{
    // TODO: Implement reverse lookup from blob_hash to nar_hash
    // For now, return None and let the caller use an empty string
    None
}

/// Extract blob hash from request path.
///
/// Path format: `/nar/{hash}.nar`
pub fn extract_blob_hash(path: &str) -> Option<&str> {
    let path = path.trim_start_matches('/');
    let path = path.strip_prefix("nar/")?;
    path.strip_suffix(".nar")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_full() {
        let range = HttpRange::parse("bytes=0-1023").unwrap();
        assert_eq!(range.start, 0);
        assert_eq!(range.end, Some(1023));
    }

    #[test]
    fn test_parse_range_open_end() {
        let range = HttpRange::parse("bytes=1024-").unwrap();
        assert_eq!(range.start, 1024);
        assert_eq!(range.end, None);
    }

    #[test]
    fn test_parse_range_invalid() {
        assert!(HttpRange::parse("invalid").is_err());
        assert!(HttpRange::parse("bytes=abc-def").is_err());
        assert!(HttpRange::parse("bytes=10").is_err());
    }

    #[test]
    fn test_resolve_range() {
        let range = HttpRange { start: 0, end: Some(99) };
        let (start, end) = range.resolve(1000).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 99);
    }

    #[test]
    fn test_resolve_range_open_end() {
        let range = HttpRange { start: 500, end: None };
        let (start, end) = range.resolve(1000).unwrap();
        assert_eq!(start, 500);
        assert_eq!(end, 999);
    }

    #[test]
    fn test_resolve_range_out_of_bounds() {
        let range = HttpRange { start: 1000, end: Some(1500) };
        assert!(range.resolve(1000).is_err());
    }

    #[test]
    fn test_extract_blob_hash() {
        assert_eq!(extract_blob_hash("/nar/abc123.nar"), Some("abc123"));
        assert_eq!(extract_blob_hash("nar/abc123.nar"), Some("abc123"));
        assert_eq!(extract_blob_hash("/nar/abc123"), None);
        assert_eq!(extract_blob_hash("/abc123.nar"), None);
    }
}
