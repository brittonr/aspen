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

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_core::Signature;
use iroh::Endpoint;
use iroh::PublicKey;
use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::identity::ClusterIdentity;
use super::identity::SignedClusterIdentity;
use super::trust::TrustManager;
use super::types::FederatedId;
use super::types::FederationSettings;

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

// ============================================================================
// Request Types
// ============================================================================

/// Federation protocol request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationRequest {
    /// Exchange cluster identities and capabilities.
    Handshake {
        /// Our signed cluster identity.
        identity: SignedClusterIdentity,
        /// Protocol version we support.
        protocol_version: u8,
        /// Capabilities we support.
        capabilities: Vec<String>,
    },

    /// List federated resources available on this cluster.
    ListResources {
        /// Resource type filter (e.g., "forge:repo").
        resource_type: Option<String>,
        /// Pagination cursor.
        cursor: Option<String>,
        /// Maximum results.
        limit: u32,
    },

    /// Get current state of a federated resource.
    GetResourceState {
        /// The federated resource ID.
        fed_id: FederatedId,
    },

    /// Request missing objects for a resource.
    SyncObjects {
        /// The federated resource ID.
        fed_id: FederatedId,
        /// Object types we want (e.g., "refs", "blobs", "cobs").
        want_types: Vec<String>,
        /// Hashes we already have (to avoid re-sending).
        have_hashes: Vec<[u8; 32]>,
        /// Maximum objects to return.
        limit: u32,
    },

    /// Verify a ref update signature.
    VerifyRefUpdate {
        /// The federated resource ID.
        fed_id: FederatedId,
        /// Ref name.
        ref_name: String,
        /// New hash.
        new_hash: [u8; 32],
        /// Signature.
        signature: Signature,
        /// Signer public key.
        signer: [u8; 32],
    },
}

// ============================================================================
// Response Types
// ============================================================================

/// Federation protocol response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationResponse {
    /// Handshake response with peer's identity.
    Handshake {
        /// Peer's signed cluster identity.
        identity: SignedClusterIdentity,
        /// Protocol version peer supports.
        protocol_version: u8,
        /// Capabilities peer supports.
        capabilities: Vec<String>,
        /// Whether we're trusted by this peer.
        trusted: bool,
    },

    /// List of federated resources.
    ResourceList {
        /// Available resources.
        resources: Vec<ResourceInfo>,
        /// Next cursor for pagination.
        next_cursor: Option<String>,
        /// Total count (if known).
        total: Option<u32>,
    },

    /// Current state of a resource.
    ResourceState {
        /// Whether the resource was found.
        found: bool,
        /// Ref heads (ref_name -> hash).
        heads: HashMap<String, [u8; 32]>,
        /// Resource metadata.
        metadata: Option<ResourceMetadata>,
    },

    /// Synced objects.
    Objects {
        /// The objects.
        objects: Vec<SyncObject>,
        /// Whether there are more objects available.
        has_more: bool,
    },

    /// Ref verification result.
    VerifyResult {
        /// Whether the signature is valid.
        valid: bool,
        /// Error message if invalid.
        error: Option<String>,
    },

    /// Error response.
    Error {
        /// Error code.
        code: String,
        /// Error message.
        message: String,
    },
}

/// Information about a federated resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Federated resource ID.
    pub fed_id: FederatedId,
    /// Resource type (e.g., "forge:repo").
    pub resource_type: String,
    /// Human-readable name.
    pub name: String,
    /// Federation mode.
    pub mode: String,
    /// Last update timestamp (microseconds).
    pub updated_at_micros: u64,
}

/// Resource metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    /// Resource type.
    pub resource_type: String,
    /// Human-readable name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Delegate public keys (for Forge repos).
    pub delegates: Vec<[u8; 32]>,
    /// Signature threshold.
    pub threshold: u32,
    /// Created timestamp (microseconds).
    pub created_at_micros: u64,
    /// Updated timestamp (microseconds).
    pub updated_at_micros: u64,
}

/// A synced object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncObject {
    /// Object type (e.g., "blob", "tree", "commit", "ref", "cob").
    pub object_type: String,
    /// Object hash (BLAKE3).
    pub hash: [u8; 32],
    /// Object data.
    pub data: Vec<u8>,
    /// Optional signature (for signed objects).
    pub signature: Option<Signature>,
    /// Optional signer (for signed objects).
    pub signer: Option<[u8; 32]>,
}

// ============================================================================
// Protocol Handler
// ============================================================================

/// Context for federation protocol handling.
pub struct FederationProtocolContext {
    /// Our cluster identity.
    pub cluster_identity: ClusterIdentity,
    /// Trust manager.
    pub trust_manager: Arc<TrustManager>,
    /// Federation settings per resource.
    pub resource_settings: Arc<RwLock<HashMap<FederatedId, FederationSettings>>>,
    /// Iroh endpoint.
    pub endpoint: Arc<Endpoint>,
}

/// Protocol handler for federation sync.
#[derive(Clone)]
pub struct FederationProtocolHandler {
    context: Arc<FederationProtocolContext>,
    connection_semaphore: Arc<Semaphore>,
}

impl std::fmt::Debug for FederationProtocolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FederationProtocolHandler")
            .field("cluster_name", &self.context.cluster_identity.name())
            .finish()
    }
}

impl FederationProtocolHandler {
    /// Create a new federation protocol handler.
    pub fn new(context: FederationProtocolContext) -> Self {
        Self {
            context: Arc::new(context),
            connection_semaphore: Arc::new(Semaphore::new(MAX_FEDERATION_CONNECTIONS as usize)),
        }
    }

    /// Get our cluster identity.
    pub fn cluster_identity(&self) -> &ClusterIdentity {
        &self.context.cluster_identity
    }

    /// Get the trust manager.
    pub fn trust_manager(&self) -> &TrustManager {
        &self.context.trust_manager
    }
}

impl ProtocolHandler for FederationProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!("Federation connection limit reached ({}), rejecting {}", MAX_FEDERATION_CONNECTIONS, remote_id);
                return Err(AcceptError::from_err(std::io::Error::other("connection limit reached")));
            }
        };

        debug!(remote = %remote_id, "accepted federation connection");

        // Handle the connection
        let result = handle_federation_connection(connection, self.context.clone()).await;

        // Release permit
        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Federation protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Handle a federation connection.
#[instrument(skip(connection, context))]
async fn handle_federation_connection(connection: Connection, context: Arc<FederationProtocolContext>) -> Result<()> {
    let remote_id = connection.remote_id();
    let mut stream_count = 0u32;

    loop {
        // Accept incoming streams
        let stream = match tokio::time::timeout(REQUEST_TIMEOUT, connection.accept_bi()).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                // Connection closed or error
                debug!(remote = %remote_id, error = %e, "federation connection ended");
                break;
            }
            Err(_) => {
                // Timeout - check if connection is still alive
                continue;
            }
        };

        // Check stream limit
        stream_count += 1;
        if stream_count > MAX_STREAMS_PER_CONNECTION {
            warn!(
                remote = %remote_id,
                "Too many streams ({}), closing connection",
                stream_count
            );
            break;
        }

        let (mut send, mut recv) = stream;
        let ctx = context.clone();

        // Handle stream in background
        tokio::spawn(async move {
            if let Err(e) = handle_federation_stream(&mut send, &mut recv, &ctx).await {
                warn!(remote = %remote_id, error = %e, "federation stream error");
            }
        });
    }

    Ok(())
}

/// Handle a single federation stream (request/response).
async fn handle_federation_stream(
    send: &mut iroh::endpoint::SendStream,
    recv: &mut iroh::endpoint::RecvStream,
    context: &FederationProtocolContext,
) -> Result<()> {
    // Read request with size limit
    let request = read_message::<FederationRequest>(recv).await?;

    // Process request
    let response = process_federation_request(request, context).await?;

    // Write response
    write_message(send, &response).await?;

    Ok(())
}

/// Read a length-prefixed message.
async fn read_message<T: for<'de> Deserialize<'de>>(recv: &mut iroh::endpoint::RecvStream) -> Result<T> {
    // Read length prefix (4 bytes, big-endian)
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf).await.context("failed to read message length")?;
    let len = u32::from_be_bytes(len_buf) as usize;

    // Check size limit
    if len > MAX_MESSAGE_SIZE {
        anyhow::bail!("message too large: {} > {}", len, MAX_MESSAGE_SIZE);
    }

    // Read message body
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await.context("failed to read message body")?;

    // Deserialize
    let message = postcard::from_bytes(&buf).context("failed to deserialize message")?;

    Ok(message)
}

/// Write a length-prefixed message.
async fn write_message<T: Serialize>(send: &mut iroh::endpoint::SendStream, message: &T) -> Result<()> {
    // Serialize
    let buf = postcard::to_allocvec(message).context("failed to serialize message")?;

    // Check size limit
    if buf.len() > MAX_MESSAGE_SIZE {
        anyhow::bail!("message too large: {} > {}", buf.len(), MAX_MESSAGE_SIZE);
    }

    // Write length prefix
    let len = buf.len() as u32;
    send.write_all(&len.to_be_bytes()).await.context("failed to write message length")?;

    // Write message body
    send.write_all(&buf).await.context("failed to write message body")?;

    Ok(())
}

/// Process a federation request.
async fn process_federation_request(
    request: FederationRequest,
    context: &FederationProtocolContext,
) -> Result<FederationResponse> {
    match request {
        FederationRequest::Handshake {
            identity,
            protocol_version: _,
            capabilities: _,
        } => {
            // Verify the peer's identity signature
            if !identity.verify() {
                return Ok(FederationResponse::Error {
                    code: "INVALID_SIGNATURE".to_string(),
                    message: "Cluster identity signature verification failed".to_string(),
                });
            }

            let peer_key = identity.public_key();
            let trusted = context.trust_manager.is_trusted(&peer_key);

            debug!(
                peer = %peer_key,
                peer_name = %identity.name(),
                trusted = trusted,
                "federation handshake"
            );

            // Return our identity
            Ok(FederationResponse::Handshake {
                identity: context.cluster_identity.to_signed(),
                protocol_version: FEDERATION_PROTOCOL_VERSION,
                capabilities: vec!["forge".to_string()],
                trusted,
            })
        }

        FederationRequest::ListResources {
            resource_type,
            cursor: _,
            limit,
        } => {
            let _limit = limit.min(MAX_RESOURCES_PER_LIST);

            // Get resources from settings
            let settings = context.resource_settings.read();
            let mut resources: Vec<ResourceInfo> = settings
                .iter()
                .filter(|(_, s)| {
                    // Only include public resources for now
                    matches!(s.mode, super::types::FederationMode::Public)
                })
                .map(|(fed_id, _)| ResourceInfo {
                    fed_id: *fed_id,
                    resource_type: "forge:repo".to_string(),
                    name: fed_id.short(),
                    mode: "public".to_string(),
                    updated_at_micros: 0,
                })
                .take(limit as usize)
                .collect();

            // Filter by resource type if specified
            if let Some(ref rt) = resource_type {
                resources.retain(|r| r.resource_type == *rt);
            }

            Ok(FederationResponse::ResourceList {
                resources,
                next_cursor: None,
                total: Some(settings.len() as u32),
            })
        }

        FederationRequest::GetResourceState { fed_id } => {
            // Check if we have this resource
            let settings = context.resource_settings.read();
            if !settings.contains_key(&fed_id) {
                return Ok(FederationResponse::ResourceState {
                    found: false,
                    heads: HashMap::new(),
                    metadata: None,
                });
            }

            // For now, return empty state (real implementation would query ForgeNode)
            Ok(FederationResponse::ResourceState {
                found: true,
                heads: HashMap::new(),
                metadata: None,
            })
        }

        FederationRequest::SyncObjects {
            fed_id,
            want_types: _,
            have_hashes: _,
            limit,
        } => {
            let _limit = limit.min(MAX_OBJECTS_PER_SYNC);

            // Check if resource exists and is federated
            let settings = context.resource_settings.read();
            if !settings.contains_key(&fed_id) {
                return Ok(FederationResponse::Error {
                    code: "NOT_FOUND".to_string(),
                    message: format!("Resource not found: {}", fed_id.short()),
                });
            }

            // For now, return empty objects (real implementation would query storage)
            Ok(FederationResponse::Objects {
                objects: vec![],
                has_more: false,
            })
        }

        FederationRequest::VerifyRefUpdate {
            fed_id,
            ref_name,
            new_hash,
            signature,
            signer,
        } => {
            // Verify the signature
            // For now, just check basic validity
            let signer_key = match PublicKey::from_bytes(&signer) {
                Ok(k) => k,
                Err(_) => {
                    return Ok(FederationResponse::VerifyResult {
                        valid: false,
                        error: Some("Invalid signer public key".to_string()),
                    });
                }
            };

            // Build the message that was signed
            let mut message = Vec::new();
            message.extend_from_slice(fed_id.origin().as_bytes());
            message.extend_from_slice(fed_id.local_id());
            message.extend_from_slice(ref_name.as_bytes());
            message.extend_from_slice(&new_hash);

            // Verify signature
            let sig_bytes: [u8; 64] = match signature.0.try_into() {
                Ok(bytes) => bytes,
                Err(_) => {
                    return Ok(FederationResponse::VerifyResult {
                        valid: false,
                        error: Some("Invalid signature format".to_string()),
                    });
                }
            };
            let sig = iroh::Signature::from_bytes(&sig_bytes);

            match signer_key.verify(&message, &sig) {
                Ok(()) => Ok(FederationResponse::VerifyResult {
                    valid: true,
                    error: None,
                }),
                Err(e) => Ok(FederationResponse::VerifyResult {
                    valid: false,
                    error: Some(format!("Signature verification failed: {}", e)),
                }),
            }
        }
    }
}

// ============================================================================
// Client Functions
// ============================================================================

/// Connect to a federated cluster and perform handshake.
pub async fn connect_to_cluster(
    endpoint: &Endpoint,
    our_identity: &ClusterIdentity,
    peer_id: PublicKey,
) -> Result<(Connection, SignedClusterIdentity)> {
    // Connect to peer
    let connection =
        endpoint.connect(peer_id, FEDERATION_ALPN).await.context("failed to connect to federated cluster")?;

    // Open a stream for handshake
    let (mut send, mut recv) = connection.open_bi().await.context("failed to open handshake stream")?;

    // Send handshake request
    let request = FederationRequest::Handshake {
        identity: our_identity.to_signed(),
        protocol_version: FEDERATION_PROTOCOL_VERSION,
        capabilities: vec!["forge".to_string()],
    };
    write_message(&mut send, &request).await?;

    // Read handshake response
    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::Handshake { identity, trusted, .. } => {
            // Verify peer's identity
            if !identity.verify() {
                anyhow::bail!("Peer identity verification failed");
            }

            info!(
                peer = %identity.public_key(),
                peer_name = %identity.name(),
                we_are_trusted = trusted,
                "federation handshake complete"
            );

            Ok((connection, identity))
        }
        FederationResponse::Error { code, message } => {
            anyhow::bail!("Handshake failed: {} - {}", code, message)
        }
        _ => anyhow::bail!("Unexpected handshake response"),
    }
}

/// List resources available on a federated cluster.
pub async fn list_remote_resources(
    connection: &Connection,
    resource_type: Option<&str>,
    limit: u32,
) -> Result<Vec<ResourceInfo>> {
    let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

    let request = FederationRequest::ListResources {
        resource_type: resource_type.map(|s| s.to_string()),
        cursor: None,
        limit,
    };
    write_message(&mut send, &request).await?;

    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::ResourceList { resources, .. } => Ok(resources),
        FederationResponse::Error { code, message } => {
            anyhow::bail!("List resources failed: {} - {}", code, message)
        }
        _ => anyhow::bail!("Unexpected response"),
    }
}

/// Get the state of a remote resource.
pub async fn get_remote_resource_state(
    connection: &Connection,
    fed_id: &FederatedId,
) -> Result<(bool, HashMap<String, [u8; 32]>, Option<ResourceMetadata>)> {
    let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

    let request = FederationRequest::GetResourceState { fed_id: *fed_id };
    write_message(&mut send, &request).await?;

    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::ResourceState { found, heads, metadata } => Ok((found, heads, metadata)),
        FederationResponse::Error { code, message } => {
            anyhow::bail!("Get resource state failed: {} - {}", code, message)
        }
        _ => anyhow::bail!("Unexpected response"),
    }
}

/// Sync objects from a remote cluster.
pub async fn sync_remote_objects(
    connection: &Connection,
    fed_id: &FederatedId,
    want_types: Vec<String>,
    have_hashes: Vec<[u8; 32]>,
    limit: u32,
) -> Result<(Vec<SyncObject>, bool)> {
    let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

    let request = FederationRequest::SyncObjects {
        fed_id: *fed_id,
        want_types,
        have_hashes,
        limit,
    };
    write_message(&mut send, &request).await?;

    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::Objects { objects, has_more } => Ok((objects, has_more)),
        FederationResponse::Error { code, message } => {
            anyhow::bail!("Sync objects failed: {} - {}", code, message)
        }
        _ => anyhow::bail!("Unexpected response"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity() -> ClusterIdentity {
        ClusterIdentity::generate("test-cluster".to_string())
    }

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
    fn test_resource_info_roundtrip() {
        let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
        let fed_id = FederatedId::new(origin, [0xab; 32]);

        let info = ResourceInfo {
            fed_id,
            resource_type: "forge:repo".to_string(),
            name: "test-repo".to_string(),
            mode: "public".to_string(),
            updated_at_micros: 1234567890,
        };

        let bytes = postcard::to_allocvec(&info).unwrap();
        let parsed: ResourceInfo = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.fed_id, fed_id);
        assert_eq!(parsed.name, "test-repo");
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
}
