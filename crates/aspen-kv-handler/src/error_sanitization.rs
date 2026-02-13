//! Security-focused error message sanitization for KV operations.
//!
//! This module provides functions to sanitize error messages before returning
//! them to clients, preventing information leakage about internal implementation.

use aspen_core::KeyValueStoreError;

/// Sanitize a KeyValueStoreError for client consumption.
///
/// Key-value errors are often user-actionable, so we preserve the error category
/// but remove potentially sensitive implementation details.
pub fn sanitize_kv_error(err: &KeyValueStoreError) -> String {
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
            format!("topology version mismatch; expected {}, got {}; refresh topology", expected, actual)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_not_found() {
        let err = KeyValueStoreError::NotFound {
            key: "test_key".to_string(),
        };
        assert_eq!(sanitize_kv_error(&err), "key not found");
    }

    #[test]
    fn test_sanitize_not_leader_with_id() {
        let err = KeyValueStoreError::NotLeader {
            leader: Some(42),
            reason: "internal reason".to_string(),
        };
        assert_eq!(sanitize_kv_error(&err), "not leader; leader is node 42");
    }

    #[test]
    fn test_sanitize_not_leader_unknown() {
        let err = KeyValueStoreError::NotLeader {
            leader: None,
            reason: "internal reason".to_string(),
        };
        assert_eq!(sanitize_kv_error(&err), "not leader; leader unknown");
    }

    #[test]
    fn test_sanitize_timeout() {
        let err = KeyValueStoreError::Timeout { duration_ms: 5000 };
        assert_eq!(sanitize_kv_error(&err), "operation timed out after 5000ms");
    }

    #[test]
    fn test_sanitize_cas_failed() {
        let err = KeyValueStoreError::CompareAndSwapFailed {
            key: "my_key".to_string(),
            expected: Some("expected_value".to_string()),
            actual: Some("actual_value".to_string()),
        };
        assert_eq!(sanitize_kv_error(&err), "compare-and-swap failed for key 'my_key'");
    }
}
