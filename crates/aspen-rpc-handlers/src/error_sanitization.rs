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

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn first_matching_message<'a>(haystack: &str, cases: &'a [(&'a [&'a str], &'a str)]) -> Option<&'a str> {
    for (needles, message) in cases {
        if contains_any(haystack, needles) {
            return Some(message);
        }
    }
    None
}

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
    let err_string = err.to_string().to_lowercase();
    let cases: &[(&[&str], &str)] = &[
        (&["not leader", "forward to leader"], "operation must be performed on leader node"),
        (&["not found", "key not found"], "resource not found"),
        (&["not initialized", "cluster not initialized", "uninitialized"], "cluster not initialized"),
        (&["timeout", "timed out"], "operation timed out"),
        (&["connection", "network", "unreachable"], "network error"),
        (&["permission", "unauthorized"], "permission denied"),
        (&["invalid", "malformed"], "invalid request"),
        (&["quorum", "membership"], "cluster membership error"),
        (&["snapshot"], "snapshot operation failed"),
        (&["storage", "database", "sqlite", "redb"], "storage error"),
    ];

    debug_assert!(!err_string.is_empty());
    let sanitized = first_matching_message(&err_string, cases).unwrap_or("internal error").to_string();
    debug_assert!(!sanitized.is_empty());
    sanitized
}

/// Sanitize an error string for client consumption.
///
/// Variant of `sanitize_error_for_client` that works with string errors.
#[allow(dead_code)]
pub fn sanitize_error_string_for_client(err: &str) -> String {
    let err_lower = err.to_lowercase();
    let cases: &[(&[&str], &str)] = &[
        (&["not leader", "forward to leader"], "operation must be performed on leader node"),
        (&["not found", "key not found"], "resource not found"),
        (&["not initialized", "cluster not initialized"], "cluster not initialized"),
        (&["timeout", "timed out"], "operation timed out"),
        (&["connection", "network"], "network error"),
        (&["invalid", "malformed"], "invalid request"),
    ];

    debug_assert!(!err_lower.is_empty() || err.is_empty());
    let sanitized = first_matching_message(&err_lower, cases).unwrap_or("internal error").to_string();
    debug_assert!(!sanitized.is_empty());
    sanitized
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

    let sanitized = match err {
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
    };

    debug_assert!(!sanitized.is_empty());
    debug_assert!(sanitized != err.to_string() || matches!(err, KeyValueStoreError::NotFound { .. }));
    sanitized
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
