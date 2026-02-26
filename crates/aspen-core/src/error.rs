//! Error types for Aspen API operations.
//!
//! Provides explicit error types with actionable context following Tiger Style.
//!
//! `ControlPlaneError` is re-exported from `aspen-cluster-types` for backward compatibility.
//! `KeyValueStoreError` is re-exported from `aspen-kv-types` for backward compatibility.
//! New code should prefer importing from the respective crates directly for lighter dependencies.

// Re-export ControlPlaneError from aspen-cluster-types
pub use aspen_cluster_types::ControlPlaneError;
// Re-export KeyValueStoreError from aspen-kv-types
pub use aspen_kv_types::KeyValueStoreError;

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // ControlPlaneError tests (now re-exported from aspen-cluster-types)
    // ============================================================================

    #[test]
    fn control_plane_error_invalid_request_display() {
        let err = ControlPlaneError::InvalidRequest {
            reason: "missing required field".to_string(),
        };
        assert_eq!(err.to_string(), "invalid request: missing required field");
    }

    #[test]
    fn control_plane_error_not_initialized_display() {
        let err = ControlPlaneError::NotInitialized;
        assert_eq!(err.to_string(), "cluster not initialized");
    }

    #[test]
    fn control_plane_error_failed_display() {
        let err = ControlPlaneError::Failed {
            reason: "network timeout".to_string(),
        };
        assert_eq!(err.to_string(), "operation failed: network timeout");
    }

    #[test]
    fn control_plane_error_unsupported_display() {
        let err = ControlPlaneError::Unsupported {
            backend: "in-memory".to_string(),
            operation: "get_metrics".to_string(),
        };
        assert_eq!(err.to_string(), "operation not supported by in-memory backend: get_metrics");
    }

    #[test]
    fn control_plane_error_timeout_display() {
        let err = ControlPlaneError::Timeout { duration_ms: 5000 };
        assert_eq!(err.to_string(), "operation timed out after 5000ms");
    }

    #[test]
    fn control_plane_error_clone() {
        let err = ControlPlaneError::InvalidRequest {
            reason: "test".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn control_plane_error_equality() {
        let err1 = ControlPlaneError::NotInitialized;
        let err2 = ControlPlaneError::NotInitialized;
        let err3 = ControlPlaneError::Timeout { duration_ms: 100 };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn control_plane_error_debug() {
        let err = ControlPlaneError::Failed {
            reason: "debug test".to_string(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("Failed"));
        assert!(debug.contains("debug test"));
    }

    // ============================================================================
    // KeyValueStoreError tests
    // ============================================================================

    #[test]
    fn kv_error_not_found_display() {
        let err = KeyValueStoreError::NotFound {
            key: "my-key".to_string(),
        };
        assert_eq!(err.to_string(), "key 'my-key' not found");
    }

    #[test]
    fn kv_error_failed_display() {
        let err = KeyValueStoreError::Failed {
            reason: "disk full".to_string(),
        };
        assert_eq!(err.to_string(), "operation failed: disk full");
    }

    #[test]
    fn kv_error_not_leader_with_leader_display() {
        let err = KeyValueStoreError::NotLeader {
            leader: Some(5),
            reason: "try again".to_string(),
        };
        assert_eq!(err.to_string(), "not leader; current leader: Some(5); try again");
    }

    #[test]
    fn kv_error_not_leader_without_leader_display() {
        let err = KeyValueStoreError::NotLeader {
            leader: None,
            reason: "election in progress".to_string(),
        };
        assert_eq!(err.to_string(), "not leader; current leader: None; election in progress");
    }

    #[test]
    fn kv_error_key_too_large_display() {
        let err = KeyValueStoreError::KeyTooLarge { size: 2048, max: 1024 };
        assert_eq!(err.to_string(), "key size 2048 exceeds maximum of 1024 bytes");
    }

    #[test]
    fn kv_error_value_too_large_display() {
        let err = KeyValueStoreError::ValueTooLarge {
            size: 2_000_000,
            max: 1_000_000,
        };
        assert_eq!(err.to_string(), "value size 2000000 exceeds maximum of 1000000 bytes");
    }

    #[test]
    fn kv_error_batch_too_large_display() {
        let err = KeyValueStoreError::BatchTooLarge { size: 200, max: 100 };
        assert_eq!(err.to_string(), "batch size 200 exceeds maximum of 100 keys");
    }

    #[test]
    fn kv_error_timeout_display() {
        let err = KeyValueStoreError::Timeout { duration_ms: 30000 };
        assert_eq!(err.to_string(), "operation timed out after 30000ms");
    }

    #[test]
    fn kv_error_compare_and_swap_failed_display() {
        let err = KeyValueStoreError::CompareAndSwapFailed {
            key: "counter".to_string(),
            expected: Some("10".to_string()),
            actual: Some("11".to_string()),
        };
        assert_eq!(
            err.to_string(),
            "compare-and-swap failed for key 'counter': expected Some(\"10\"), found Some(\"11\")"
        );
    }

    #[test]
    fn kv_error_compare_and_swap_failed_none_values_display() {
        let err = KeyValueStoreError::CompareAndSwapFailed {
            key: "new-key".to_string(),
            expected: None,
            actual: Some("exists".to_string()),
        };
        assert!(err.to_string().contains("expected None"));
        assert!(err.to_string().contains("found Some"));
    }

    #[test]
    fn kv_error_empty_key_display() {
        let err = KeyValueStoreError::EmptyKey;
        assert_eq!(err.to_string(), "key cannot be empty");
    }

    #[test]
    fn kv_error_shard_moved_display() {
        let err = KeyValueStoreError::ShardMoved {
            key: "user:123".to_string(),
            new_shard_id: 5,
            topology_version: 42,
        };
        assert_eq!(err.to_string(), "key 'user:123' moved to shard 5 (topology version 42)");
    }

    #[test]
    fn kv_error_shard_not_ready_display() {
        let err = KeyValueStoreError::ShardNotReady {
            shard_id: 3,
            state: "migrating".to_string(),
        };
        assert_eq!(err.to_string(), "shard 3 is migrating, operation not allowed");
    }

    #[test]
    fn kv_error_topology_version_mismatch_display() {
        let err = KeyValueStoreError::TopologyVersionMismatch {
            expected: 10,
            actual: 15,
        };
        assert_eq!(err.to_string(), "topology version mismatch: expected 10, got 15");
    }

    #[test]
    fn kv_error_clone() {
        let err = KeyValueStoreError::NotFound {
            key: "test".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn kv_error_equality() {
        let err1 = KeyValueStoreError::EmptyKey;
        let err2 = KeyValueStoreError::EmptyKey;
        let err3 = KeyValueStoreError::Timeout { duration_ms: 100 };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn kv_error_equality_with_same_fields() {
        let err1 = KeyValueStoreError::NotFound {
            key: "same".to_string(),
        };
        let err2 = KeyValueStoreError::NotFound {
            key: "same".to_string(),
        };
        let err3 = KeyValueStoreError::NotFound {
            key: "different".to_string(),
        };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn kv_error_debug() {
        let err = KeyValueStoreError::KeyTooLarge { size: 5000, max: 1024 };
        let debug = format!("{:?}", err);
        assert!(debug.contains("KeyTooLarge"));
        assert!(debug.contains("5000"));
        assert!(debug.contains("1024"));
    }
}
