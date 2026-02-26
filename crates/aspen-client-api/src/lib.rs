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

    // =========================================================================
    // Postcard wire-format stability tests
    //
    // Regression: `aspen-client-api` features (`ci`, `secrets`, `automerge`)
    // used to default to OFF. When the CLI was built with different features
    // than the server, `#[cfg(feature)]` variants in ClientRpcRequest /
    // ClientRpcResponse shifted postcard discriminants, causing deserialization
    // crashes ("Found a bool that wasn't 0 or 1").
    //
    // Fix: all features are now default-on so the enum layout is always
    // identical.  These tests make sure it stays that way.
    // =========================================================================

    /// All features that gate enum variants must be default-on.
    /// If a new `#[cfg(feature = "X")]` is added to a variant, add "X" here.
    #[test]
    fn test_all_enum_layout_features_are_default() {
        // These features gate variants inside ClientRpcRequest / ClientRpcResponse.
        // They MUST be in [features] default so every build sees the same layout.
        //
        // If you add a new cfg(feature)-gated variant, add the feature here AND
        // to the `default` list in Cargo.toml.
        #[cfg(not(feature = "ci"))]
        compile_error!("feature `ci` must be default-on for postcard layout stability");
        #[cfg(not(feature = "secrets"))]
        compile_error!("feature `secrets` must be default-on for postcard layout stability");
        #[cfg(not(feature = "automerge"))]
        compile_error!("feature `automerge` must be default-on for postcard layout stability");
        #[cfg(not(feature = "auth"))]
        compile_error!("feature `auth` must be default-on for postcard layout stability");
    }

    /// Postcard roundtrip for every feature-gated response variant.
    /// If any variant is compiled-out, postcard discriminants shift and this
    /// test catches it because the variant won't exist at all.
    #[test]
    fn test_feature_gated_response_variants_postcard_roundtrip() {
        // CI-gated variants
        let ci_resp = ClientRpcResponse::CacheMigrationStartResult(ci::CacheMigrationStartResultResponse {
            started: true,
            status: None,
            error: None,
        });
        let bytes = postcard::to_stdvec(&ci_resp).expect("ci serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("ci deserialize");
        assert!(matches!(decoded, ClientRpcResponse::CacheMigrationStartResult(_)));

        // Automerge-gated variants
        let am_resp = ClientRpcResponse::AutomergeCreateResult(automerge::AutomergeCreateResultResponse {
            is_success: true,
            document_id: Some("doc1".into()),
            error: None,
        });
        let bytes = postcard::to_stdvec(&am_resp).expect("automerge serialize");
        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("automerge deserialize");
        assert!(matches!(decoded, ClientRpcResponse::AutomergeCreateResult(_)));

        // Secrets-gated request variant roundtrip (no response variant is
        // directly secrets-gated, but the request enum has them)
        let sec_req = ClientRpcRequest::SecretsKvRead {
            mount: "secret".into(),
            path: "db/password".into(),
            version: None,
        };
        let bytes = postcard::to_stdvec(&sec_req).expect("secrets serialize");
        let decoded: ClientRpcRequest = postcard::from_bytes(&bytes).expect("secrets deserialize");
        assert_eq!(decoded.variant_name(), "SecretsKvRead");
    }

    /// Postcard roundtrip for core (non-gated) variants that sit at both ends
    /// of the enum. If feature-gated variants in the middle are removed, the
    /// discriminant for these will shift and this catches it.
    #[test]
    fn test_first_and_last_response_variants_postcard_stable() {
        // First variant: Health
        let first = ClientRpcResponse::Health(HealthResponse {
            status: "healthy".into(),
            node_id: 1,
            raft_node_id: Some(1),
            uptime_seconds: 100,
            is_initialized: true,
            membership_node_count: Some(3),
        });
        let first_bytes = postcard::to_stdvec(&first).expect("first serialize");
        // The first variant's postcard discriminant should be 0
        assert_eq!(first_bytes[0], 0, "Health must be discriminant 0");

        let decoded: ClientRpcResponse = postcard::from_bytes(&first_bytes).expect("first deserialize");
        assert!(matches!(decoded, ClientRpcResponse::Health(_)));

        // Last non-gated variant before feature-gated section: CapabilityUnavailable
        let cap = ClientRpcResponse::CapabilityUnavailable(CapabilityUnavailableResponse {
            required_app: "test".into(),
            message: "not loaded".into(),
            hints: vec![],
        });
        let cap_bytes = postcard::to_stdvec(&cap).expect("cap serialize");
        let cap_decoded: ClientRpcResponse = postcard::from_bytes(&cap_bytes).expect("cap deserialize");
        assert!(matches!(cap_decoded, ClientRpcResponse::CapabilityUnavailable(_)));

        // Very last variant: PluginReloadResult
        let last = ClientRpcResponse::PluginReloadResult(PluginReloadResultResponse {
            is_success: true,
            plugin_count: 0,
            error: None,
            message: "ok".into(),
        });
        let last_bytes = postcard::to_stdvec(&last).expect("last serialize");
        let last_decoded: ClientRpcResponse = postcard::from_bytes(&last_bytes).expect("last deserialize");
        assert!(matches!(last_decoded, ClientRpcResponse::PluginReloadResult(_)));
    }

    /// The Error variant must always have the same discriminant so the CLI
    /// retry logic (`e.code == "NOT_LEADER"`) works regardless of features.
    #[test]
    fn test_error_response_discriminant_is_stable() {
        let err = ClientRpcResponse::error("NOT_LEADER", "try another");
        let bytes = postcard::to_stdvec(&err).expect("serialize");
        // Error is variant index 14 (0-indexed) — if this changes, the
        // CLI's retry loop may break because it deserializes the wrong type.
        // Update this value if you intentionally reorder variants BEFORE Error.
        let discriminant = bytes[0];

        let decoded: ClientRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");
        match decoded {
            ClientRpcResponse::Error(e) => {
                assert_eq!(e.code, "NOT_LEADER");
            }
            other => panic!("Error discriminant {discriminant} decoded as {other:?}"),
        }
    }

    // =========================================================================
    // Original tests
    // =========================================================================

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
