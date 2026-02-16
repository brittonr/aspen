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
        BlobStoreError::DeleteTag { .. } | BlobStoreError::ListTags { .. } | BlobStoreError::SetTag { .. } => {
            "tag operation failed".to_string()
        }
        BlobStoreError::AddBytes { .. } | BlobStoreError::AddPath { .. } => "failed to add blob".to_string(),
        BlobStoreError::ReadFileMetadata { .. } => "failed to read file".to_string(),
        BlobStoreError::CheckExistence { .. } => "storage error".to_string(),
        BlobStoreError::ListBlobs { .. } => "failed to list blobs".to_string(),
        BlobStoreError::DownloadBlob { .. } => "download failed".to_string(),
    }
}
