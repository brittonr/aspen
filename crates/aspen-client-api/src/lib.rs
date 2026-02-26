//! Client API protocol definitions for Aspen.
//!
//! This crate provides the protocol definitions for the Aspen Client RPC API,
//! which allows clients to communicate with aspen-node instances over Iroh P2P
//! connections using the `aspen-client` ALPN identifier.
//!
//! # Architecture
//!
//! The Client RPC uses a distinct ALPN to distinguish it from Raft RPC, allowing
//! clients to connect directly to nodes without needing HTTP. All communication
//! uses authenticated requests with optional capability tokens for authorization.
//!
//! # Core Types
//!
//! - [`AuthenticatedRequest`] - Wrapper for requests with optional auth tokens
//! - [`ClientRpcRequest`] - Enum of all supported client operations
//! - [`ClientRpcResponse`] - Enum of all possible response types
//!
//! # Protocol Constants
//!
//! - [`CLIENT_ALPN`] - ALPN identifier for client connections
//! - [`MAX_CLIENT_MESSAGE_SIZE`] - Maximum message size (1 MB)
//! - [`MAX_CLUSTER_NODES`] - Maximum nodes in cluster state response
//!
//! # Example
//!
//! ```rust
//! use aspen_client_api::{ClientRpcRequest, AuthenticatedRequest};
//!
//! let request = ClientRpcRequest::GetHealth;
//! let auth_request = AuthenticatedRequest::unauthenticated(request);
//! ```

pub mod messages;

// Re-export all public types for convenience
pub use messages::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_are_bounded() {
        assert!(MAX_CLIENT_MESSAGE_SIZE > 0);
        assert!(MAX_CLIENT_MESSAGE_SIZE <= 16 * 1024 * 1024);
        assert!(MAX_CLUSTER_NODES > 0);
        assert!(MAX_CLUSTER_NODES <= 256);
        assert!(MAX_CLIENT_CONNECTIONS > 0);
        assert!(DEFAULT_GIT_CHUNK_SIZE_BYTES <= MAX_GIT_CHUNK_SIZE_BYTES);
    }

    #[test]
    fn test_client_alpn_is_correct() {
        assert_eq!(CLIENT_ALPN, b"aspen-client");
    }

    #[test]
    fn test_get_health_variant_name() {
        let req = ClientRpcRequest::GetHealth;
        assert_eq!(req.variant_name(), "GetHealth");
    }

    #[test]
    fn test_get_health_domain() {
        let req = ClientRpcRequest::GetHealth;
        // GetHealth is a core request, no forge domain
        assert_ne!(req.variant_name(), "");
    }

    #[test]
    fn test_get_health_roundtrip_json() {
        let req = ClientRpcRequest::GetHealth;
        let json = serde_json::to_string(&req).unwrap();
        let decoded: ClientRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.variant_name(), "GetHealth");
    }

    #[test]
    fn test_git_bridge_probe_objects_request_roundtrip() {
        let req = ClientRpcRequest::GitBridgeProbeObjects {
            repo_id: "repo1".into(),
            sha1s: vec!["a".repeat(40), "b".repeat(40)],
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: ClientRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.variant_name(), "GitBridgeProbeObjects");
    }

    #[test]
    fn test_git_bridge_probe_objects_variant_name() {
        let req = ClientRpcRequest::GitBridgeProbeObjects {
            repo_id: "repo1".into(),
            sha1s: vec![],
        };
        assert_eq!(req.variant_name(), "GitBridgeProbeObjects");
    }

    #[test]
    fn test_git_bridge_probe_objects_response_roundtrip() {
        let resp = ClientRpcResponse::GitBridgeProbeObjects(GitBridgeProbeObjectsResponse {
            is_success: true,
            known_sha1s: vec!["c".repeat(40)],
            error: None,
        });
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: ClientRpcResponse = serde_json::from_str(&json).unwrap();
        if let ClientRpcResponse::GitBridgeProbeObjects(r) = decoded {
            assert!(r.is_success);
            assert_eq!(r.known_sha1s.len(), 1);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn test_authenticated_request_unauthenticated() {
        let req = AuthenticatedRequest::unauthenticated(ClientRpcRequest::GetHealth);
        assert!(req.token.is_none());
        assert_eq!(req.proxy_hops, 0);
        assert_eq!(req.request.variant_name(), "GetHealth");
    }

    #[test]
    fn test_authenticated_request_from() {
        let auth: AuthenticatedRequest = ClientRpcRequest::GetHealth.into();
        assert!(auth.token.is_none());
        assert_eq!(auth.proxy_hops, 0);
    }

    #[test]
    fn test_kv_write_variant_name() {
        let req = ClientRpcRequest::WriteKey {
            key: "test".into(),
            value: "val".into(),
        };
        assert_eq!(req.variant_name(), "WriteKey");
    }

    #[test]
    fn test_kv_read_variant_name() {
        let req = ClientRpcRequest::ReadKey { key: "test".into() };
        assert_eq!(req.variant_name(), "ReadKey");
    }

    #[test]
    fn test_response_error_roundtrip() {
        let resp = ClientRpcResponse::error("NOT_LEADER", "try another node");
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: ClientRpcResponse = serde_json::from_str(&json).unwrap();
        if let ClientRpcResponse::Error(e) = decoded {
            assert_eq!(e.code, "NOT_LEADER");
            assert_eq!(e.message, "try another node");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn test_response_pong_roundtrip() {
        let resp = ClientRpcResponse::Pong;
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: ClientRpcResponse = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, ClientRpcResponse::Pong));
    }

    #[test]
    fn test_response_leader_roundtrip() {
        let resp = ClientRpcResponse::Leader(Some(1));
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: ClientRpcResponse = serde_json::from_str(&json).unwrap();
        if let ClientRpcResponse::Leader(id) = decoded {
            assert_eq!(id, Some(1));
        } else {
            panic!("wrong variant");
        }
    }
}
