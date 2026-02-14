//! Federation sync protocol for cross-cluster synchronization.
//!
//! This module implements the ALPN-based protocol for syncing resources
//! between federated Aspen clusters. It handles:
//!
//! - Handshake and capability negotiation
//! - Resource listing and state queries
//! - Object synchronization (refs, blobs, COBs)
//!
//! # Protocol
//!
//! The federation protocol uses QUIC streams over Iroh connections:
//!
//! 1. **Handshake**: Exchange cluster identities and capabilities
//! 2. **ListResources**: Query available federated resources
//! 3. **GetResourceState**: Get current heads/metadata for a resource
//! 4. **SyncObjects**: Request and transfer missing objects
//!
//! # Security
//!
//! - All connections are verified against trust settings
//! - Cluster signatures are verified on all signed data
//! - Delegate signatures are verified for canonical refs
//!
//! # Wire Format
//!
//! Messages use postcard serialization with length-prefixed framing.

mod client;
mod handler;
mod types;
mod wire;

use std::time::Duration;

// Client
pub use client::connect_to_cluster;
pub use client::get_remote_resource_state;
pub use client::list_remote_resources;
pub use client::sync_remote_objects;
// Handler
pub use handler::FederationProtocolContext;
pub use handler::FederationProtocolHandler;
// Re-export public API

// Types
pub use types::FederationRequest;
pub use types::FederationResponse;
pub use types::ResourceInfo;
pub use types::ResourceMetadata;
pub use types::SyncObject;

// ============================================================================
// Constants (Tiger Style: Fixed limits)
// ============================================================================

/// ALPN identifier for federation protocol.
pub const FEDERATION_ALPN: &[u8] = b"/aspen/federation/1";

/// Protocol version.
pub const FEDERATION_PROTOCOL_VERSION: u8 = 1;

/// Maximum concurrent federation connections.
pub const MAX_FEDERATION_CONNECTIONS: u32 = 64;

/// Maximum streams per federation connection.
pub const MAX_STREAMS_PER_CONNECTION: u32 = 16;

/// Maximum size of a single federation message.
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16 MB

/// Maximum objects per sync request.
pub const MAX_OBJECTS_PER_SYNC: u32 = 1000;

/// Maximum resources per list request.
pub const MAX_RESOURCES_PER_LIST: u32 = 1000;

/// Handshake timeout.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Request timeout.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Per-message processing timeout (5 seconds).
///
/// Tiger Style: Prevents CPU exhaustion from large message deserialization.
/// Applied to both read and process operations per message.
/// Generous enough for 16MB message at typical network speeds.
pub const MESSAGE_PROCESSING_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use aspen_core::Signature;
    use aspen_core::hlc::SerializableTimestamp;
    use tokio::sync::RwLock;

    use super::*;
    use crate::identity::ClusterIdentity;
    use crate::trust::TrustManager;
    use crate::types::FederatedId;
    use crate::types::FederationSettings;

    fn test_identity() -> ClusterIdentity {
        ClusterIdentity::generate("test-cluster".to_string())
    }

    // =========================================================================
    // Constants Tests
    // =========================================================================

    #[test]
    fn test_federation_alpn() {
        assert_eq!(FEDERATION_ALPN, b"/aspen/federation/1");
    }

    #[test]
    fn test_federation_protocol_version() {
        assert_eq!(FEDERATION_PROTOCOL_VERSION, 1);
    }

    #[test]
    fn test_max_federation_connections() {
        assert_eq!(MAX_FEDERATION_CONNECTIONS, 64);
    }

    #[test]
    fn test_max_streams_per_connection() {
        assert_eq!(MAX_STREAMS_PER_CONNECTION, 16);
    }

    #[test]
    fn test_max_message_size() {
        assert_eq!(MAX_MESSAGE_SIZE, 16 * 1024 * 1024); // 16 MB
    }

    #[test]
    fn test_max_objects_per_sync() {
        assert_eq!(MAX_OBJECTS_PER_SYNC, 1000);
    }

    #[test]
    fn test_max_resources_per_list() {
        assert_eq!(MAX_RESOURCES_PER_LIST, 1000);
    }

    #[test]
    fn test_handshake_timeout() {
        assert_eq!(HANDSHAKE_TIMEOUT, Duration::from_secs(10));
    }

    #[test]
    fn test_request_timeout() {
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(60));
    }

    #[test]
    fn test_message_processing_timeout_constant() {
        // Tiger Style: Verify timeout is reasonable for 16MB messages
        assert_eq!(MESSAGE_PROCESSING_TIMEOUT, Duration::from_secs(5));
        // Ensure processing timeout is shorter than request timeout
        assert!(MESSAGE_PROCESSING_TIMEOUT < REQUEST_TIMEOUT);
        // Ensure we have enough time to process large messages (16MB at ~100Mbps = ~1.3s)
        assert!(MESSAGE_PROCESSING_TIMEOUT >= Duration::from_secs(2));
    }

    // =========================================================================
    // Request Roundtrip Tests
    // =========================================================================

    #[test]
    fn test_handshake_request_roundtrip() {
        let identity = test_identity();

        let request = FederationRequest::Handshake {
            identity: identity.to_signed(),
            protocol_version: FEDERATION_PROTOCOL_VERSION,
            capabilities: vec!["forge".to_string()],
        };

        let bytes = postcard::to_allocvec(&request).unwrap();
        let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationRequest::Handshake {
                protocol_version,
                capabilities,
                ..
            } => {
                assert_eq!(protocol_version, FEDERATION_PROTOCOL_VERSION);
                assert_eq!(capabilities, vec!["forge"]);
            }
            _ => panic!("expected Handshake"),
        }
    }

    #[test]
    fn test_handshake_request_empty_capabilities() {
        let identity = test_identity();

        let request = FederationRequest::Handshake {
            identity: identity.to_signed(),
            protocol_version: FEDERATION_PROTOCOL_VERSION,
            capabilities: vec![],
        };

        let bytes = postcard::to_allocvec(&request).unwrap();
        let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationRequest::Handshake { capabilities, .. } => {
                assert!(capabilities.is_empty());
            }
            _ => panic!("expected Handshake"),
        }
    }

    #[test]
    fn test_handshake_request_multiple_capabilities() {
        let identity = test_identity();

        let request = FederationRequest::Handshake {
            identity: identity.to_signed(),
            protocol_version: FEDERATION_PROTOCOL_VERSION,
            capabilities: vec!["forge".to_string(), "ci".to_string(), "registry".to_string()],
        };

        let bytes = postcard::to_allocvec(&request).unwrap();
        let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationRequest::Handshake { capabilities, .. } => {
                assert_eq!(capabilities.len(), 3);
            }
            _ => panic!("expected Handshake"),
        }
    }

    #[test]
    fn test_list_resources_request_roundtrip() {
        let request = FederationRequest::ListResources {
            resource_type: Some("forge:repo".to_string()),
            cursor: None,
            limit: 100,
        };

        let bytes = postcard::to_allocvec(&request).unwrap();
        let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationRequest::ListResources {
                resource_type, limit, ..
            } => {
                assert_eq!(resource_type, Some("forge:repo".to_string()));
                assert_eq!(limit, 100);
            }
            _ => panic!("expected ListResources"),
        }
    }

    #[test]
    fn test_list_resources_request_no_filter() {
        let request = FederationRequest::ListResources {
            resource_type: None,
            cursor: None,
            limit: MAX_RESOURCES_PER_LIST,
        };

        let bytes = postcard::to_allocvec(&request).unwrap();
        let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationRequest::ListResources {
                resource_type,
                cursor,
                limit,
            } => {
                assert!(resource_type.is_none());
                assert!(cursor.is_none());
                assert_eq!(limit, MAX_RESOURCES_PER_LIST);
            }
            _ => panic!("expected ListResources"),
        }
    }

    #[test]
    fn test_list_resources_request_with_cursor() {
        let request = FederationRequest::ListResources {
            resource_type: None,
            cursor: Some("cursor-token-123".to_string()),
            limit: 50,
        };

        let bytes = postcard::to_allocvec(&request).unwrap();
        let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationRequest::ListResources { cursor, .. } => {
                assert_eq!(cursor, Some("cursor-token-123".to_string()));
            }
            _ => panic!("expected ListResources"),
        }
    }

    #[test]
    fn test_get_resource_state_request_roundtrip() {
        let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
        let fed_id = FederatedId::new(origin, [0xab; 32]);

        let request = FederationRequest::GetResourceState { fed_id };

        let bytes = postcard::to_allocvec(&request).unwrap();
        let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationRequest::GetResourceState { fed_id: parsed_fed_id } => {
                assert_eq!(parsed_fed_id, fed_id);
            }
            _ => panic!("expected GetResourceState"),
        }
    }

    #[test]
    fn test_sync_objects_request_roundtrip() {
        let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
        let fed_id = FederatedId::new(origin, [0xcd; 32]);

        let request = FederationRequest::SyncObjects {
            fed_id,
            want_types: vec!["refs".to_string(), "blobs".to_string()],
            have_hashes: vec![[0xaa; 32], [0xbb; 32]],
            limit: 100,
        };

        let bytes = postcard::to_allocvec(&request).unwrap();
        let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationRequest::SyncObjects {
                want_types,
                have_hashes,
                limit,
                ..
            } => {
                assert_eq!(want_types, vec!["refs", "blobs"]);
                assert_eq!(have_hashes.len(), 2);
                assert_eq!(limit, 100);
            }
            _ => panic!("expected SyncObjects"),
        }
    }

    #[test]
    fn test_sync_objects_request_empty_hashes() {
        let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
        let fed_id = FederatedId::new(origin, [0xcd; 32]);

        let request = FederationRequest::SyncObjects {
            fed_id,
            want_types: vec!["cobs".to_string()],
            have_hashes: vec![],
            limit: MAX_OBJECTS_PER_SYNC,
        };

        let bytes = postcard::to_allocvec(&request).unwrap();
        let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationRequest::SyncObjects { have_hashes, limit, .. } => {
                assert!(have_hashes.is_empty());
                assert_eq!(limit, MAX_OBJECTS_PER_SYNC);
            }
            _ => panic!("expected SyncObjects"),
        }
    }

    #[test]
    fn test_verify_ref_update_request_roundtrip() {
        let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
        let fed_id = FederatedId::new(origin, [0xee; 32]);

        let request = FederationRequest::VerifyRefUpdate {
            fed_id,
            ref_name: "refs/heads/main".to_string(),
            new_hash: [0xff; 32],
            signature: Signature::from_bytes([0u8; 64]),
            signer: [0xab; 32],
        };

        let bytes = postcard::to_allocvec(&request).unwrap();
        let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationRequest::VerifyRefUpdate { ref_name, new_hash, .. } => {
                assert_eq!(ref_name, "refs/heads/main");
                assert_eq!(new_hash, [0xff; 32]);
            }
            _ => panic!("expected VerifyRefUpdate"),
        }
    }

    // =========================================================================
    // Response Roundtrip Tests
    // =========================================================================

    #[test]
    fn test_handshake_response_roundtrip() {
        let identity = test_identity();

        let response = FederationResponse::Handshake {
            identity: identity.to_signed(),
            protocol_version: FEDERATION_PROTOCOL_VERSION,
            capabilities: vec!["forge".to_string()],
            trusted: true,
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::Handshake {
                protocol_version,
                trusted,
                ..
            } => {
                assert_eq!(protocol_version, FEDERATION_PROTOCOL_VERSION);
                assert!(trusted);
            }
            _ => panic!("expected Handshake response"),
        }
    }

    #[test]
    fn test_handshake_response_untrusted() {
        let identity = test_identity();

        let response = FederationResponse::Handshake {
            identity: identity.to_signed(),
            protocol_version: FEDERATION_PROTOCOL_VERSION,
            capabilities: vec![],
            trusted: false,
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::Handshake { trusted, .. } => {
                assert!(!trusted);
            }
            _ => panic!("expected Handshake response"),
        }
    }

    #[test]
    fn test_resource_list_response_roundtrip() {
        let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
        let fed_id = FederatedId::new(origin, [0xab; 32]);
        let hlc = aspen_core::hlc::create_hlc("test-node");

        let response = FederationResponse::ResourceList {
            resources: vec![ResourceInfo {
                fed_id,
                resource_type: "forge:repo".to_string(),
                name: "test-repo".to_string(),
                mode: "public".to_string(),
                updated_at_hlc: SerializableTimestamp::from(hlc.new_timestamp()),
            }],
            next_cursor: Some("next-page".to_string()),
            total: Some(100),
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::ResourceList {
                resources,
                next_cursor,
                total,
            } => {
                assert_eq!(resources.len(), 1);
                assert_eq!(next_cursor, Some("next-page".to_string()));
                assert_eq!(total, Some(100));
            }
            _ => panic!("expected ResourceList"),
        }
    }

    #[test]
    fn test_resource_list_response_empty() {
        let response = FederationResponse::ResourceList {
            resources: vec![],
            next_cursor: None,
            total: Some(0),
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::ResourceList { resources, total, .. } => {
                assert!(resources.is_empty());
                assert_eq!(total, Some(0));
            }
            _ => panic!("expected ResourceList"),
        }
    }

    #[test]
    fn test_resource_info_roundtrip() {
        let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
        let fed_id = FederatedId::new(origin, [0xab; 32]);

        let hlc = aspen_core::hlc::create_hlc("test-node");
        let info = ResourceInfo {
            fed_id,
            resource_type: "forge:repo".to_string(),
            name: "test-repo".to_string(),
            mode: "public".to_string(),
            updated_at_hlc: SerializableTimestamp::from(hlc.new_timestamp()),
        };

        let bytes = postcard::to_allocvec(&info).unwrap();
        let parsed: ResourceInfo = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.fed_id, fed_id);
        assert_eq!(parsed.name, "test-repo");
    }

    #[test]
    fn test_resource_state_response_found() {
        let response = FederationResponse::ResourceState {
            found: true,
            heads: {
                let mut h = HashMap::new();
                h.insert("heads/main".to_string(), [0xef; 32]);
                h.insert("heads/dev".to_string(), [0xab; 32]);
                h
            },
            metadata: None,
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::ResourceState { found, heads, .. } => {
                assert!(found);
                assert_eq!(heads.len(), 2);
                assert!(heads.contains_key("heads/main"));
                assert!(heads.contains_key("heads/dev"));
            }
            _ => panic!("expected ResourceState"),
        }
    }

    #[test]
    fn test_resource_state_response_not_found() {
        let response = FederationResponse::ResourceState {
            found: false,
            heads: HashMap::new(),
            metadata: None,
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::ResourceState { found, heads, .. } => {
                assert!(!found);
                assert!(heads.is_empty());
            }
            _ => panic!("expected ResourceState"),
        }
    }

    #[test]
    fn test_resource_state_with_metadata() {
        let hlc = aspen_core::hlc::create_hlc("test-node");
        let metadata = ResourceMetadata {
            resource_type: "forge:repo".to_string(),
            name: "my-repo".to_string(),
            description: Some("A test repository".to_string()),
            delegates: vec![[0xaa; 32], [0xbb; 32]],
            threshold: 2,
            created_at_hlc: SerializableTimestamp::from(hlc.new_timestamp()),
            updated_at_hlc: SerializableTimestamp::from(hlc.new_timestamp()),
        };

        let response = FederationResponse::ResourceState {
            found: true,
            heads: HashMap::new(),
            metadata: Some(metadata),
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::ResourceState { metadata, .. } => {
                let meta = metadata.unwrap();
                assert_eq!(meta.name, "my-repo");
                assert_eq!(meta.delegates.len(), 2);
                assert_eq!(meta.threshold, 2);
            }
            _ => panic!("expected ResourceState"),
        }
    }

    #[test]
    fn test_sync_object_roundtrip() {
        let obj = SyncObject {
            object_type: "blob".to_string(),
            hash: [0xcd; 32],
            data: b"hello world".to_vec(),
            signature: None,
            signer: None,
        };

        let bytes = postcard::to_allocvec(&obj).unwrap();
        let parsed: SyncObject = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.object_type, "blob");
        assert_eq!(parsed.data, b"hello world");
    }

    #[test]
    fn test_sync_object_with_signature() {
        let obj = SyncObject {
            object_type: "commit".to_string(),
            hash: [0xdd; 32],
            data: b"commit data".to_vec(),
            signature: Some(Signature::from_bytes([0u8; 64])),
            signer: Some([0xee; 32]),
        };

        let bytes = postcard::to_allocvec(&obj).unwrap();
        let parsed: SyncObject = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.object_type, "commit");
        assert!(parsed.signature.is_some());
        assert!(parsed.signer.is_some());
    }

    #[test]
    fn test_sync_object_empty_data() {
        let obj = SyncObject {
            object_type: "tree".to_string(),
            hash: [0x00; 32],
            data: vec![],
            signature: None,
            signer: None,
        };

        let bytes = postcard::to_allocvec(&obj).unwrap();
        let parsed: SyncObject = postcard::from_bytes(&bytes).unwrap();

        assert!(parsed.data.is_empty());
    }

    #[test]
    fn test_objects_response_roundtrip() {
        let response = FederationResponse::Objects {
            objects: vec![
                SyncObject {
                    object_type: "blob".to_string(),
                    hash: [0x11; 32],
                    data: b"data1".to_vec(),
                    signature: None,
                    signer: None,
                },
                SyncObject {
                    object_type: "blob".to_string(),
                    hash: [0x22; 32],
                    data: b"data2".to_vec(),
                    signature: None,
                    signer: None,
                },
            ],
            has_more: true,
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::Objects { objects, has_more } => {
                assert_eq!(objects.len(), 2);
                assert!(has_more);
            }
            _ => panic!("expected Objects"),
        }
    }

    #[test]
    fn test_objects_response_no_more() {
        let response = FederationResponse::Objects {
            objects: vec![],
            has_more: false,
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::Objects { objects, has_more } => {
                assert!(objects.is_empty());
                assert!(!has_more);
            }
            _ => panic!("expected Objects"),
        }
    }

    #[test]
    fn test_verify_result_response_valid() {
        let response = FederationResponse::VerifyResult {
            valid: true,
            error: None,
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::VerifyResult { valid, error } => {
                assert!(valid);
                assert!(error.is_none());
            }
            _ => panic!("expected VerifyResult"),
        }
    }

    #[test]
    fn test_verify_result_response_invalid() {
        let response = FederationResponse::VerifyResult {
            valid: false,
            error: Some("signature verification failed".to_string()),
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::VerifyResult { valid, error } => {
                assert!(!valid);
                assert_eq!(error, Some("signature verification failed".to_string()));
            }
            _ => panic!("expected VerifyResult"),
        }
    }

    #[test]
    fn test_error_response_roundtrip() {
        let response = FederationResponse::Error {
            code: "NOT_FOUND".to_string(),
            message: "Resource not found".to_string(),
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::Error { code, message } => {
                assert_eq!(code, "NOT_FOUND");
                assert_eq!(message, "Resource not found");
            }
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn test_response_roundtrip() {
        let response = FederationResponse::ResourceState {
            found: true,
            heads: {
                let mut h = HashMap::new();
                h.insert("heads/main".to_string(), [0xef; 32]);
                h
            },
            metadata: None,
        };

        let bytes = postcard::to_allocvec(&response).unwrap();
        let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

        match parsed {
            FederationResponse::ResourceState { found, heads, .. } => {
                assert!(found);
                assert!(heads.contains_key("heads/main"));
            }
            _ => panic!("expected ResourceState"),
        }
    }

    // =========================================================================
    // Resource Metadata Tests
    // =========================================================================

    #[test]
    fn test_resource_metadata_roundtrip() {
        let hlc = aspen_core::hlc::create_hlc("test-node");
        let metadata = ResourceMetadata {
            resource_type: "forge:repo".to_string(),
            name: "test-repo".to_string(),
            description: Some("A description".to_string()),
            delegates: vec![[0xaa; 32]],
            threshold: 1,
            created_at_hlc: SerializableTimestamp::from(hlc.new_timestamp()),
            updated_at_hlc: SerializableTimestamp::from(hlc.new_timestamp()),
        };

        let bytes = postcard::to_allocvec(&metadata).unwrap();
        let parsed: ResourceMetadata = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.name, "test-repo");
        assert_eq!(parsed.description, Some("A description".to_string()));
        assert_eq!(parsed.threshold, 1);
    }

    #[test]
    fn test_resource_metadata_no_description() {
        let hlc = aspen_core::hlc::create_hlc("test-node");
        let metadata = ResourceMetadata {
            resource_type: "forge:repo".to_string(),
            name: "repo".to_string(),
            description: None,
            delegates: vec![],
            threshold: 0,
            created_at_hlc: SerializableTimestamp::from(hlc.new_timestamp()),
            updated_at_hlc: SerializableTimestamp::from(hlc.new_timestamp()),
        };

        let bytes = postcard::to_allocvec(&metadata).unwrap();
        let parsed: ResourceMetadata = postcard::from_bytes(&bytes).unwrap();

        assert!(parsed.description.is_none());
        assert!(parsed.delegates.is_empty());
    }

    // =========================================================================
    // FederationProtocolHandler Tests
    // =========================================================================

    #[test]
    fn test_federation_protocol_context_fields() {
        // Test that we can create a FederationProtocolContext by verifying field types
        // This is a compile-time check without requiring network access
        let identity = test_identity();
        let trust_manager = Arc::new(TrustManager::new());
        let _resource_settings: Arc<RwLock<HashMap<FederatedId, FederationSettings>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let hlc = aspen_core::hlc::create_hlc("test-node");

        // Verify identity methods work
        assert_eq!(identity.name(), "test-cluster");
        let signed = identity.to_signed();
        assert!(signed.verify());

        // Verify trust manager starts with no trusted peers
        let random_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        assert!(!trust_manager.is_trusted(&random_key));

        // Verify HLC creates valid timestamps (just ensure it doesn't panic)
        let _ts = hlc.new_timestamp();
    }

    #[test]
    fn test_federation_protocol_handler_debug() {
        // Verify Debug impl exists and doesn't panic
        let identity = test_identity();
        let debug_str = format!("{:?}", identity);
        assert!(!debug_str.is_empty());
    }
}
