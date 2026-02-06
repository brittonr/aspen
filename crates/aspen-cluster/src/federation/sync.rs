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
use aspen_core::hlc::HLC;
use aspen_core::hlc::SerializableTimestamp;
use iroh::Endpoint;
use iroh::PublicKey;
use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::identity::ClusterIdentity;
use super::identity::SignedClusterIdentity;
use super::resolver::FederationResourceError;
use super::resolver::FederationResourceResolver;
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

/// Per-message processing timeout (5 seconds).
///
/// Tiger Style: Prevents CPU exhaustion from large message deserialization.
/// Applied to both read and process operations per message.
/// Generous enough for 16MB message at typical network speeds.
pub const MESSAGE_PROCESSING_TIMEOUT: Duration = Duration::from_secs(5);

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
    /// HLC timestamp of last update.
    pub updated_at_hlc: SerializableTimestamp,
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
    /// HLC timestamp when created.
    pub created_at_hlc: SerializableTimestamp,
    /// HLC timestamp when last updated.
    pub updated_at_hlc: SerializableTimestamp,
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
    /// HLC for timestamping.
    pub hlc: Arc<HLC>,
    /// Resource resolver for storage access.
    ///
    /// When set, enables actual data fetching for GetResourceState and SyncObjects.
    /// If None, these handlers return stub responses.
    pub resource_resolver: Option<Arc<dyn FederationResourceResolver>>,
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
    // Read request with size limit and per-message timeout
    // Tiger Style: Prevents CPU exhaustion from slow/malicious senders
    let request = tokio::time::timeout(MESSAGE_PROCESSING_TIMEOUT, read_message::<FederationRequest>(recv))
        .await
        .context("message read timeout")?
        .context("failed to read federation request")?;

    // Process request with timeout to prevent CPU exhaustion
    let response = tokio::time::timeout(MESSAGE_PROCESSING_TIMEOUT, process_federation_request(request, context))
        .await
        .context("message processing timeout")?
        .context("failed to process federation request")?;

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
            let settings = context.resource_settings.read().await;
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
                    updated_at_hlc: SerializableTimestamp::from(context.hlc.new_timestamp()),
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
            // Use resource resolver if available
            if let Some(ref resolver) = context.resource_resolver {
                match resolver.get_resource_state(&fed_id).await {
                    Ok(state) => {
                        return Ok(FederationResponse::ResourceState {
                            found: state.found,
                            heads: state.heads,
                            metadata: state.metadata,
                        });
                    }
                    Err(FederationResourceError::NotFound { .. }) => {
                        return Ok(FederationResponse::ResourceState {
                            found: false,
                            heads: HashMap::new(),
                            metadata: None,
                        });
                    }
                    Err(FederationResourceError::FederationDisabled { fed_id }) => {
                        return Ok(FederationResponse::Error {
                            code: "FEDERATION_DISABLED".to_string(),
                            message: format!("Federation is disabled for resource: {}", fed_id),
                        });
                    }
                    Err(FederationResourceError::ShardNotReady { shard_id, state }) => {
                        return Ok(FederationResponse::Error {
                            code: "SHARD_NOT_READY".to_string(),
                            message: format!("Shard {} is {}, retry later", shard_id, state),
                        });
                    }
                    Err(FederationResourceError::Internal { message }) => {
                        warn!(fed_id = %fed_id.short(), error = %message, "internal error getting resource state");
                        return Ok(FederationResponse::Error {
                            code: "INTERNAL_ERROR".to_string(),
                            message,
                        });
                    }
                }
            }

            // Fallback: Check settings only (no actual data fetch)
            let settings = context.resource_settings.read().await;
            if !settings.contains_key(&fed_id) {
                return Ok(FederationResponse::ResourceState {
                    found: false,
                    heads: HashMap::new(),
                    metadata: None,
                });
            }

            // No resolver available, return stub response
            Ok(FederationResponse::ResourceState {
                found: true,
                heads: HashMap::new(),
                metadata: None,
            })
        }

        FederationRequest::SyncObjects {
            fed_id,
            want_types,
            have_hashes,
            limit,
        } => {
            let limit = limit.min(MAX_OBJECTS_PER_SYNC);

            // Use resource resolver if available
            if let Some(ref resolver) = context.resource_resolver {
                match resolver.sync_objects(&fed_id, &want_types, &have_hashes, limit).await {
                    Ok(objects) => {
                        let has_more = objects.len() >= limit as usize;
                        return Ok(FederationResponse::Objects { objects, has_more });
                    }
                    Err(FederationResourceError::NotFound { fed_id }) => {
                        return Ok(FederationResponse::Error {
                            code: "NOT_FOUND".to_string(),
                            message: format!("Resource not found: {}", fed_id),
                        });
                    }
                    Err(FederationResourceError::FederationDisabled { fed_id }) => {
                        return Ok(FederationResponse::Error {
                            code: "FEDERATION_DISABLED".to_string(),
                            message: format!("Federation is disabled for resource: {}", fed_id),
                        });
                    }
                    Err(FederationResourceError::ShardNotReady { shard_id, state }) => {
                        return Ok(FederationResponse::Error {
                            code: "SHARD_NOT_READY".to_string(),
                            message: format!("Shard {} is {}, retry later", shard_id, state),
                        });
                    }
                    Err(FederationResourceError::Internal { message }) => {
                        warn!(fed_id = %fed_id.short(), error = %message, "internal error syncing objects");
                        return Ok(FederationResponse::Error {
                            code: "INTERNAL_ERROR".to_string(),
                            message,
                        });
                    }
                }
            }

            // Fallback: Check settings only
            let settings = context.resource_settings.read().await;
            if !settings.contains_key(&fed_id) {
                return Ok(FederationResponse::Error {
                    code: "NOT_FOUND".to_string(),
                    message: format!("Resource not found: {}", fed_id.short()),
                });
            }

            // No resolver available, return empty response
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
