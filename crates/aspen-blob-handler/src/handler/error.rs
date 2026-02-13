//! Error sanitization for blob operations.

use aspen_blob::BlobStoreError;

/// Sanitize a blob store error for client consumption.
///
/// Blob store errors can contain file paths, IO errors, and other internal details.
/// We categorize them into user-safe messages.
pub(crate) fn sanitize_blob_error(err: &BlobStoreError) -> String {
    match err {
        BlobStoreError::NotFound { .. } => "blob not found".to_string(),
        BlobStoreError::TooLarge { max, .. } => format!("blob too large; max {} bytes", max),
        BlobStoreError::Storage { .. } => "storage error".to_string(),
        BlobStoreError::Download { .. } => "download failed".to_string(),
        BlobStoreError::InvalidTicket { .. } => "invalid ticket".to_string(),
    }
}
