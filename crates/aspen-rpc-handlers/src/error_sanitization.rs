//! Security-focused error message sanitization.
//!
//! This module provides functions to sanitize error messages before returning
//! them to clients, preventing information leakage about internal implementation.
//!
//! # Security Rationale
//!
//! Internal errors can reveal:
//! - File paths and system structure
//! - Database schema and query patterns
//! - Network topology and peer information
//! - Internal component names and versions
//!
//! By categorizing errors and returning generic messages, we prevent
//! information leakage while still providing useful feedback to clients.

/// Sanitize an error message for client consumption.
///
/// This function converts internal error messages to user-safe messages that
/// don't leak implementation details, file paths, or internal state.
///
/// # Tiger Style
///
/// - Exhaustive pattern matching for known error types
/// - Generic fallback for unknown errors
/// - Full error logged internally at appropriate level
pub fn sanitize_error_for_client(err: &anyhow::Error) -> String {
    // Check for specific error types we can provide better messages for
    let err_string = err.to_string().to_lowercase();

    // Categorize errors by their root cause
    if err_string.contains("not leader") || err_string.contains("forward to leader") {
        return "operation must be performed on leader node".to_string();
    }

    if err_string.contains("not found") || err_string.contains("key not found") {
        return "resource not found".to_string();
    }

    if err_string.contains("not initialized")
        || err_string.contains("cluster not initialized")
        || err_string.contains("uninitialized")
    {
        return "cluster not initialized".to_string();
    }

    if err_string.contains("timeout") || err_string.contains("timed out") {
        return "operation timed out".to_string();
    }

    if err_string.contains("connection") || err_string.contains("network") || err_string.contains("unreachable") {
        return "network error".to_string();
    }

    if err_string.contains("permission") || err_string.contains("unauthorized") {
        return "permission denied".to_string();
    }

    if err_string.contains("invalid") || err_string.contains("malformed") {
        return "invalid request".to_string();
    }

    if err_string.contains("quorum") || err_string.contains("membership") {
        return "cluster membership error".to_string();
    }

    if err_string.contains("snapshot") {
        return "snapshot operation failed".to_string();
    }

    if err_string.contains("storage")
        || err_string.contains("database")
        || err_string.contains("sqlite")
        || err_string.contains("redb")
    {
        return "storage error".to_string();
    }

    // Generic fallback - never expose raw error messages
    "internal error".to_string()
}

/// Sanitize an error string for client consumption.
///
/// Variant of `sanitize_error_for_client` that works with string errors.
#[allow(dead_code)]
pub fn sanitize_error_string_for_client(err: &str) -> String {
    let err_lower = err.to_lowercase();

    if err_lower.contains("not leader") || err_lower.contains("forward to leader") {
        return "operation must be performed on leader node".to_string();
    }

    if err_lower.contains("not found") || err_lower.contains("key not found") {
        return "resource not found".to_string();
    }

    if err_lower.contains("not initialized") || err_lower.contains("cluster not initialized") {
        return "cluster not initialized".to_string();
    }

    if err_lower.contains("timeout") || err_lower.contains("timed out") {
        return "operation timed out".to_string();
    }

    if err_lower.contains("connection") || err_lower.contains("network") {
        return "network error".to_string();
    }

    if err_lower.contains("invalid") || err_lower.contains("malformed") {
        return "invalid request".to_string();
    }

    // Generic fallback
    "internal error".to_string()
}

/// Sanitize a ControlPlaneError for client consumption.
///
/// These errors are part of our API and can be returned directly since they
/// are already designed to be user-facing. However, we still sanitize the
/// inner reason strings to be safe.
pub fn sanitize_control_error(err: &aspen_core::ControlPlaneError) -> String {
    use aspen_core::ControlPlaneError;
    match err {
        ControlPlaneError::InvalidRequest { .. } => "invalid request".to_string(),
        ControlPlaneError::NotInitialized => "cluster not initialized".to_string(),
        ControlPlaneError::Failed { .. } => "operation failed".to_string(),
        ControlPlaneError::Unsupported { operation, .. } => {
            format!("operation not supported: {}", operation)
        }
        ControlPlaneError::Timeout { duration_ms } => {
            format!("operation timed out after {}ms", duration_ms)
        }
    }
}

/// Sanitize a KeyValueStoreError for client consumption.
///
/// Key-value errors are often user-actionable, so we preserve the error category
/// but remove potentially sensitive implementation details.
pub fn sanitize_kv_error(err: &aspen_core::KeyValueStoreError) -> String {
    use aspen_core::KeyValueStoreError;
    match err {
        KeyValueStoreError::NotFound { .. } => "key not found".to_string(),
        KeyValueStoreError::Failed { .. } => "operation failed".to_string(),
        KeyValueStoreError::NotLeader { leader, .. } => {
            if let Some(leader_id) = leader {
                format!("not leader; leader is node {}", leader_id)
            } else {
                "not leader; leader unknown".to_string()
            }
        }
        KeyValueStoreError::KeyTooLarge { max, .. } => {
            format!("key too large; max {} bytes", max)
        }
        KeyValueStoreError::ValueTooLarge { max, .. } => {
            format!("value too large; max {} bytes", max)
        }
        KeyValueStoreError::BatchTooLarge { max, .. } => {
            format!("batch too large; max {} keys", max)
        }
        KeyValueStoreError::Timeout { duration_ms } => {
            format!("operation timed out after {}ms", duration_ms)
        }
        KeyValueStoreError::CompareAndSwapFailed { key, .. } => {
            format!("compare-and-swap failed for key '{}'", key)
        }
        KeyValueStoreError::EmptyKey => "key cannot be empty".to_string(),
        KeyValueStoreError::ShardMoved {
            new_shard_id,
            topology_version,
            ..
        } => {
            format!("key moved to shard {}; update topology (version {})", new_shard_id, topology_version)
        }
        KeyValueStoreError::ShardNotReady { shard_id, state } => {
            format!("shard {} is {}; retry later", shard_id, state)
        }
        KeyValueStoreError::TopologyVersionMismatch { expected, actual } => {
            format!("topology version mismatch: expected {}, got {}", expected, actual)
        }
    }
}

/// Sanitize a blob store error for client consumption.
///
/// Blob store errors can contain file paths, IO errors, and other internal details.
/// We categorize them into user-safe messages.
#[cfg(feature = "blob")]
pub fn sanitize_blob_error(err: &aspen_blob::BlobStoreError) -> String {
    use aspen_blob::BlobStoreError;
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
