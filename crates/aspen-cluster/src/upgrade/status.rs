//! Deployment status reporting to cluster KV.
//!
//! Writes per-node status transitions to `_sys:deploy:node:{node_id}`
//! so the deployment coordinator can track progress.

use aspen_deploy::DEPLOY_NODE_PREFIX;
use aspen_deploy::NodeDeployStatus;

/// Format a KV key for per-node deployment status.
///
/// Key: `_sys:deploy:node:{node_id}`
#[inline]
pub fn node_status_key(node_id: u64) -> String {
    format!("{DEPLOY_NODE_PREFIX}{node_id}")
}

/// Serialize a node deploy status for KV storage.
pub fn serialize_status(status: &NodeDeployStatus) -> String {
    serde_json::to_string(status).unwrap_or_else(|_| {
        // Fallback: use the status string directly.
        format!("\"{}\"", status.as_status_str())
    })
}

/// Deserialize a node deploy status from KV storage.
pub fn deserialize_status(value: &str) -> Option<NodeDeployStatus> {
    serde_json::from_str(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_status_key() {
        assert_eq!(node_status_key(1), "_sys:deploy:node:1");
        assert_eq!(node_status_key(42), "_sys:deploy:node:42");
        assert_eq!(node_status_key(0), "_sys:deploy:node:0");
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let statuses = vec![
            NodeDeployStatus::Pending,
            NodeDeployStatus::Draining,
            NodeDeployStatus::Upgrading,
            NodeDeployStatus::Restarting,
            NodeDeployStatus::Healthy,
            NodeDeployStatus::Failed("timeout".into()),
        ];

        for status in statuses {
            let serialized = serialize_status(&status);
            let deserialized = deserialize_status(&serialized);
            assert_eq!(deserialized, Some(status.clone()), "roundtrip failed for {serialized}");
        }
    }

    #[test]
    fn test_deserialize_invalid() {
        assert!(deserialize_status("not json").is_none());
        assert!(deserialize_status("").is_none());
    }

    #[test]
    fn test_serialize_all_variants() {
        // Verify all variants produce valid JSON.
        let pending = serialize_status(&NodeDeployStatus::Pending);
        assert!(pending.contains("pending"));

        let failed = serialize_status(&NodeDeployStatus::Failed("disk full".into()));
        assert!(failed.contains("disk full"));
    }
}
